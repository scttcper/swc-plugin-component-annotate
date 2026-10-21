pub mod config;
mod jsx_scope;
mod jsx_utils;
pub mod path_utils;
mod styled;

use config::PluginConfig;
use jsx_scope::{
    collect_function_param_scope, collect_pat_identifiers_with_status, collect_pat_list_scope,
    JsxBindingTracker, JsxIdentifierStatus,
};
use jsx_utils::*;
use path_utils::{extract_absolute_path, extract_filename};
use rustc_hash::{FxHashMap, FxHashSet};
use styled::{styled_call_component_ref, transform_styled_call, StyledTransformAttrs};
use swc_core::{
    common::{errors::HANDLER, FileName, DUMMY_SP},
    ecma::{
        ast::*,
        atoms::Atom,
        visit::{noop_visit_mut_type, noop_visit_type, Visit, VisitMut, VisitMutWith, VisitWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ComponentAnnotationPolicy {
    Normal,
    Ignored,
    Transparent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AttributeInsertionPosition {
    Append,
    // JSX spreads are order-sensitive. Transparent callsite metadata must sit
    // before a forwarded props spread so owner attrs can pass through wrappers.
    BeforeFirstSpread,
}

struct ElementAttrs {
    attrs: Vec<JSXAttrOrSpread>,
    insertion_position: AttributeInsertionPosition,
}

type DefinitionId = usize;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DefinitionRole {
    /// JSX the component returns
    ReturnRoot,
    /// JSX an `<svg>` renders, which gets no attributes
    SvgContent,
    Other,
}

#[derive(Default)]
struct ReturnRootDefinitions {
    roots: FxHashSet<DefinitionId>,
    svg_contents: FxHashSet<DefinitionId>,
    definition_count: usize,
    next_definition: DefinitionId,
}

impl ReturnRootDefinitions {
    fn next_role(&mut self) -> DefinitionRole {
        if self.next_definition >= self.definition_count {
            return DefinitionRole::Other;
        }

        let definition = self.next_definition;
        self.next_definition += 1;
        if self.roots.contains(&definition) {
            DefinitionRole::ReturnRoot
        } else if self.svg_contents.contains(&definition) {
            DefinitionRole::SvgContent
        } else {
            DefinitionRole::Other
        }
    }
}

struct Definition {
    sources: Vec<Id>,
    contains_jsx: bool,
}

// React Compiler caches JSX in local definitions before returning it. Resolve
// aliases to the latest earlier definition that contains JSX or forwards
// another identifier. Cache restore assignments such as `t1 = $[0]` are not
// value origins and are intentionally skipped.
#[derive(Default)]
struct ReturnRootDefinitionCollector {
    definitions: Vec<Definition>,
    definitions_by_binding: FxHashMap<Id, Vec<DefinitionId>>,
    returned_values: Vec<(Id, DefinitionId)>,
    /// Bindings an `<svg>` renders, like `t0` in `<svg>{t0}</svg>`
    svg_values: Vec<(Id, DefinitionId)>,
}

impl ReturnRootDefinitionCollector {
    fn add_definition(&mut self, target: &BindingIdent, value: &Expr) {
        let id = self.definitions.len();
        let mut sources = Vec::new();
        let contains_jsx = collect_value_sources(value, &mut sources);
        self.definitions.push(Definition {
            sources,
            contains_jsx,
        });
        self.definitions_by_binding
            .entry(target.id.to_id())
            .or_default()
            .push(id);
    }

    fn add_returned_value(&mut self, value: &Expr) {
        let before = self.definitions.len();
        let mut sources = Vec::new();
        collect_value_sources(value, &mut sources);
        self.returned_values
            .extend(sources.into_iter().map(|source| (source, before)));
    }

    fn resolve(&self, binding: &Id, before: DefinitionId, resolved: &mut FxHashSet<DefinitionId>) {
        let Some(definition) = self
            .definitions_by_binding
            .get(binding)
            .and_then(|definitions| {
                definitions.iter().rev().copied().find(|definition| {
                    *definition < before
                        && (self.definitions[*definition].contains_jsx
                            || !self.definitions[*definition].sources.is_empty())
                })
            })
        else {
            return;
        };

        if !resolved.insert(definition) {
            return;
        }
        for source in &self.definitions[definition].sources {
            self.resolve(source, definition, resolved);
        }
    }

    fn finish(self) -> ReturnRootDefinitions {
        let mut roots = FxHashSet::default();
        for (binding, before) in &self.returned_values {
            self.resolve(binding, *before, &mut roots);
        }

        // The compiler hoists `<svg>` children into definitions ahead of it
        let mut svg_contents = FxHashSet::default();
        for (binding, before) in &self.svg_values {
            self.resolve(binding, *before, &mut svg_contents);
        }
        // JSX the component also returns outside an `<svg>` stays annotated
        svg_contents.retain(|definition| !roots.contains(definition));

        ReturnRootDefinitions {
            roots,
            svg_contents,
            definition_count: self.definitions.len(),
            next_definition: 0,
        }
    }
}

fn collect_value_sources(value: &Expr, sources: &mut Vec<Id>) -> bool {
    match value {
        Expr::Ident(ident) => {
            sources.push(ident.to_id());
            false
        }
        Expr::JSXElement(element) => {
            collect_jsx_element_sources(element, sources);
            true
        }
        Expr::JSXFragment(fragment) => {
            collect_jsx_child_sources(&fragment.children, sources);
            true
        }
        Expr::Bin(binary)
            if matches!(
                binary.op,
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
            ) =>
        {
            collect_value_sources(&binary.left, sources)
                | collect_value_sources(&binary.right, sources)
        }
        Expr::Cond(cond) => {
            collect_value_sources(&cond.cons, sources) | collect_value_sources(&cond.alt, sources)
        }
        Expr::Paren(paren) => collect_value_sources(&paren.expr, sources),
        Expr::Seq(seq) => seq
            .exprs
            .last()
            .is_some_and(|value| collect_value_sources(value, sources)),
        // Calls, objects and arrays pass nested JSX into what the component renders,
        // e.g. `tct('[link]', {link: <Link />})` or `createPortal(<div />, el)`.
        // Callees and member objects stay out so `$[0]` cache reads aren't origins.
        Expr::Call(call) => collect_args_sources(&call.args, sources),
        // A callback handed to a rendered call, like `items.map(t3)`, renders its JSX
        Expr::Arrow(_) | Expr::Fn(_) => contains_jsx(value),
        Expr::New(new) => new
            .args
            .as_deref()
            .is_some_and(|args| collect_args_sources(args, sources)),
        Expr::Array(array) => array
            .elems
            .iter()
            .flatten()
            .fold(false, |contains_jsx, elem| {
                collect_value_sources(&elem.expr, sources) | contains_jsx
            }),
        Expr::Object(object) => object.props.iter().fold(false, |contains_jsx, prop| {
            let value_contains_jsx = match prop {
                PropOrSpread::Spread(spread) => collect_value_sources(&spread.expr, sources),
                PropOrSpread::Prop(prop) => match prop.as_ref() {
                    Prop::KeyValue(key_value) => collect_value_sources(&key_value.value, sources),
                    Prop::Shorthand(ident) => {
                        sources.push(ident.to_id());
                        false
                    }
                    _ => false,
                },
                #[cfg(swc_ast_unknown)]
                _ => false,
            };
            value_contains_jsx | contains_jsx
        }),
        _ => false,
    }
}

fn collect_args_sources(args: &[ExprOrSpread], sources: &mut Vec<Id>) -> bool {
    args.iter().fold(false, |contains_jsx, arg| {
        collect_value_sources(&arg.expr, sources) | contains_jsx
    })
}

fn contains_jsx(expr: &Expr) -> bool {
    #[derive(Default)]
    struct JsxFinder {
        found: bool,
    }

    impl Visit for JsxFinder {
        noop_visit_type!();

        fn visit_jsx_element(&mut self, _: &JSXElement) {
            self.found = true;
        }

        fn visit_jsx_fragment(&mut self, _: &JSXFragment) {
            self.found = true;
        }
    }

    let mut finder = JsxFinder::default();
    expr.visit_with(&mut finder);
    finder.found
}

fn collect_jsx_element_sources(element: &JSXElement, sources: &mut Vec<Id>) {
    // Nothing inside an <svg> is annotated, so JSX it renders isn't a root
    if !is_svg_element_name(&element.opening.name) {
        collect_jsx_child_sources(&element.children, sources);
    }
}

fn collect_jsx_child_sources(children: &[JSXElementChild], sources: &mut Vec<Id>) {
    for child in children {
        match child {
            JSXElementChild::JSXExprContainer(container) => {
                if let JSXExpr::Expr(expr) = &container.expr {
                    collect_value_sources(expr, sources);
                }
            }
            JSXElementChild::JSXSpreadChild(spread) => {
                collect_value_sources(&spread.expr, sources);
            }
            JSXElementChild::JSXElement(element) => {
                collect_jsx_element_sources(element, sources);
            }
            JSXElementChild::JSXFragment(fragment) => {
                collect_jsx_child_sources(&fragment.children, sources);
            }
            JSXElementChild::JSXText(_) => {}
            #[cfg(swc_ast_unknown)]
            _ => {}
        }
    }
}

impl Visit for ReturnRootDefinitionCollector {
    noop_visit_type!();

    fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}

    fn visit_function(&mut self, _: &Function) {}

    fn visit_jsx_element(&mut self, element: &JSXElement) {
        if is_svg_element_name(&element.opening.name) {
            let before = self.definitions.len();
            let mut sources = Vec::new();
            collect_jsx_child_sources(&element.children, &mut sources);
            self.svg_values
                .extend(sources.into_iter().map(|source| (source, before)));
        }
        element.visit_children_with(self);
    }

    fn visit_assign_expr(&mut self, assign_expr: &AssignExpr) {
        let definition = matches!(
            &assign_expr.left,
            AssignTarget::Simple(SimpleAssignTarget::Ident(_))
        ) && assign_expr.op == AssignOp::Assign;
        if definition {
            let AssignTarget::Simple(SimpleAssignTarget::Ident(target)) = &assign_expr.left else {
                unreachable!();
            };
            self.add_definition(target, &assign_expr.right);
        }
        assign_expr.visit_children_with(self);
    }

    fn visit_return_stmt(&mut self, return_stmt: &ReturnStmt) {
        if let Some(value) = return_stmt.arg.as_deref() {
            value.visit_with(self);
            self.add_returned_value(value);
        }
    }

    fn visit_var_declarator(&mut self, var_declarator: &VarDeclarator) {
        if let (Pat::Ident(target), Some(value)) = (&var_declarator.name, &var_declarator.init) {
            self.add_definition(target, value);
        }
        var_declarator.visit_children_with(self);
    }
}

fn return_root_definitions_from_function_body(body: &FunctionBody) -> ReturnRootDefinitions {
    let mut collector = ReturnRootDefinitionCollector::default();
    body.visit_with(&mut collector);
    collector.finish()
}

fn return_root_definitions_from_function(function: &Function) -> ReturnRootDefinitions {
    function
        .body
        .as_ref()
        .map(return_root_definitions_from_function_body)
        .unwrap_or_default()
}

fn return_root_definitions_from_arrow(arrow_expr: &ArrowExpr) -> ReturnRootDefinitions {
    match arrow_expr.body.as_ref() {
        ArrowFunctionBody::FunctionBody(body) => return_root_definitions_from_function_body(body),
        ArrowFunctionBody::Expr(expr) => {
            let mut collector = ReturnRootDefinitionCollector::default();
            expr.visit_with(&mut collector);
            collector.add_returned_value(expr);
            collector.finish()
        }
        #[cfg(swc_ast_unknown)]
        _ => ReturnRootDefinitions::default(),
    }
}

fn commonjs_require_source(expr: &Expr) -> Option<&str> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Callee::Expr(callee) = &call.callee else {
        return None;
    };
    if !matches!(callee.as_ref(), Expr::Ident(ident) if ident.sym.as_ref() == "require") {
        return None;
    }
    let [argument] = call.args.as_slice() else {
        return None;
    };
    if argument.spread.is_some() {
        return None;
    }
    let Expr::Lit(Lit::Str(source)) = argument.expr.as_ref() else {
        return None;
    };
    source.value.as_str()
}

/// Looks through parens and TypeScript `as`/`satisfies` around a component
/// definition, e.g. `const Button = ((props) => <button />) as FC<Props>`.
fn unwrap_component_expr(expr: &mut Expr) -> &mut Expr {
    match expr {
        Expr::Paren(ParenExpr { expr, .. })
        | Expr::TsAs(TsAsExpr { expr, .. })
        | Expr::TsSatisfies(TsSatisfiesExpr { expr, .. }) => unwrap_component_expr(expr),
        _ => expr,
    }
}

pub struct ReactComponentAnnotateVisitor {
    config: PluginConfig,
    source_file_name: Option<Str>,
    source_file_path: Option<Str>,
    current_component_name: Option<Atom>,
    fragment_child_component_name: Option<Atom>,
    current_return_component_name: Option<Atom>,
    return_root_definitions: ReturnRootDefinitions,
    react_compiler_enabled: bool,
    ignored_components_set: FxHashSet<String>,
    transparent_components_set: FxHashSet<String>,
    jsx_bindings: JsxBindingTracker,
    fragment_component_identifiers: FxHashSet<Id>,
    react_memo_identifiers: FxHashSet<Id>,
    react_namespace_identifiers: FxHashSet<Id>,
    component_attr_ident: IdentName,
    element_attr_ident: IdentName,
    source_file_attr_ident: IdentName,
    source_path_attr_ident: Option<IdentName>,
    /// Track the local identifier name for `styled` from @emotion/styled
    styled_import: Option<Id>,
    /// Owner a callback lends to what it returns, captured when the callback is
    /// entered from rendered output, like a `.map` inside returned JSX.
    callback_render_owner: Option<Atom>,
    /// Owner transparent callsites fall back to right now. Only set while visiting
    /// what a render callback returns, so other JSX in its body stays unowned.
    active_callback_owner: Option<Atom>,
    /// React Compiler outlines callbacks into module-level `_temp` helpers. Maps each
    /// helper to the component that rendered it, or `None` when more than one did.
    temp_helper_owners: FxHashMap<Atom, Option<Atom>>,
    /// Set while visiting what an `<svg>` renders. Paths, groups and anything else
    /// in there get no attributes.
    in_svg: bool,
    /// `_temp` helpers an `<svg>` renders, which get no attributes either
    svg_temp_helpers: FxHashSet<Atom>,
}

impl ReactComponentAnnotateVisitor {
    pub fn new(config: PluginConfig, filename: &FileName) -> Self {
        let source_file_name = extract_filename(filename).map(|value| Str {
            span: DUMMY_SP,
            value: value.into(),
            raw: None,
        });
        let source_file_path = extract_absolute_path(filename).map(|value| Str {
            span: DUMMY_SP,
            value: value.into(),
            raw: None,
        });

        // Pre-compute ignored components set for O(1) lookups
        let ignored_components_set: FxHashSet<String> =
            config.ignored_components.iter().cloned().collect();
        let transparent_components_set: FxHashSet<String> =
            config.transparent_components.iter().cloned().collect();
        let component_attr_ident = IdentName::new(config.component_attr_name().into(), DUMMY_SP);
        let element_attr_ident = IdentName::new(config.element_attr_name().into(), DUMMY_SP);
        let source_file_attr_ident =
            IdentName::new(config.source_file_attr_name().into(), DUMMY_SP);
        let source_path_attr_ident = config
            .source_path_attr
            .as_ref()
            .map(|_| IdentName::new(config.source_path_attr_name().into(), DUMMY_SP));
        Self {
            component_attr_ident,
            config,
            element_attr_ident,
            ignored_components_set,
            transparent_components_set,
            jsx_bindings: JsxBindingTracker::new(),
            fragment_component_identifiers: FxHashSet::default(),
            react_namespace_identifiers: FxHashSet::default(),
            source_file_name,
            source_file_attr_ident,
            source_file_path,
            source_path_attr_ident,
            current_component_name: None,
            fragment_child_component_name: None,
            current_return_component_name: None,
            return_root_definitions: ReturnRootDefinitions::default(),
            react_compiler_enabled: false,
            styled_import: None,
            react_memo_identifiers: FxHashSet::default(),
            callback_render_owner: None,
            active_callback_owner: None,
            temp_helper_owners: FxHashMap::default(),
            in_svg: false,
            svg_temp_helpers: FxHashSet::default(),
        }
    }

    #[inline]
    pub fn should_ignore_component(&self, component_name: &str) -> bool {
        self.ignored_components_set.contains(component_name)
    }

    #[inline]
    pub fn should_treat_component_as_transparent(&self, component_name: &str) -> bool {
        self.transparent_components_set.contains(component_name)
    }

    #[inline]
    fn component_annotation_policy(&self, component_name: &str) -> ComponentAnnotationPolicy {
        if self.should_ignore_component(component_name) {
            ComponentAnnotationPolicy::Ignored
        } else if self.should_treat_component_as_transparent(component_name) {
            ComponentAnnotationPolicy::Transparent
        } else {
            ComponentAnnotationPolicy::Normal
        }
    }

    #[inline]
    fn should_skip_component_return(&self, component_name: &str) -> bool {
        self.component_annotation_policy(component_name) != ComponentAnnotationPolicy::Normal
    }

    #[inline]
    fn should_skip_component_child_traversal(&self, component_name: &str) -> bool {
        // Transparent component bodies should not stamp their implementation
        // file onto forwarded DOM; callers provide the useful owner metadata.
        self.component_annotation_policy(component_name) == ComponentAnnotationPolicy::Transparent
    }

    fn element_attrs(
        &self,
        element_name: &str,
        element_policy: ComponentAnnotationPolicy,
        existing_attrs: &AttributePresence,
        transparent_owner_component_name: Option<&Atom>,
        report_element: bool,
    ) -> Option<ElementAttrs> {
        if let Some(ref component_name) = self.current_component_name {
            if self.should_skip_component_return(component_name) {
                return None;
            }
        }

        if element_policy == ComponentAnnotationPolicy::Ignored {
            return None;
        }

        let can_annotate_element =
            report_element && element_policy == ComponentAnnotationPolicy::Normal;
        let owner_component_name = if element_policy == ComponentAnnotationPolicy::Transparent {
            transparent_owner_component_name
        } else {
            self.current_component_name.as_ref()
        };
        let has_owner_component = owner_component_name.is_some();

        let mut attrs = Vec::with_capacity(4);

        if can_annotate_element
            && !existing_attrs.element
            && (self.config.component_attr_name() != self.config.element_attr_name()
                || !has_owner_component)
        {
            attrs.push(create_jsx_attr_with_ident(
                &self.element_attr_ident,
                element_name,
            ));
        }

        if !existing_attrs.component {
            if let Some(component_name) = owner_component_name {
                attrs.push(create_jsx_attr_with_ident(
                    &self.component_attr_ident,
                    component_name.as_ref(),
                ));
            }
        }

        if !existing_attrs.source_file && (has_owner_component || can_annotate_element) {
            if let Some(ref source_file) = self.source_file_name {
                attrs.push(create_jsx_attr_with_ident_and_str(
                    &self.source_file_attr_ident,
                    source_file,
                ));
            }
        }

        if !existing_attrs.source_path && (has_owner_component || can_annotate_element) {
            if let (Some(ref source_path), Some(ref source_path_attr_ident)) =
                (&self.source_file_path, &self.source_path_attr_ident)
            {
                attrs.push(create_jsx_attr_with_ident_and_str(
                    source_path_attr_ident,
                    source_path,
                ));
            }
        }

        if attrs.is_empty() {
            return None;
        }

        Some(ElementAttrs {
            attrs,
            insertion_position: if element_policy == ComponentAnnotationPolicy::Transparent {
                AttributeInsertionPosition::BeforeFirstSpread
            } else {
                AttributeInsertionPosition::Append
            },
        })
    }

    fn transparent_owner_component_name(&self, element_name: &str) -> Option<Atom> {
        let owner_component_name = self
            .current_component_name
            .as_ref()
            .or(self.fragment_child_component_name.as_ref())
            .or(self.active_callback_owner.as_ref())?;
        let owner_component_name = owner_component_name.as_ref();
        let mut transparent_owner =
            String::with_capacity(owner_component_name.len() + 1 + element_name.len());
        transparent_owner.push_str(owner_component_name);
        transparent_owner.push('-');
        transparent_owner.push_str(element_name);
        Some(transparent_owner.into())
    }

    #[inline]
    fn is_unannotatable_identifier(&self, ident: &Ident) -> bool {
        let id = ident.to_id();

        match self.scoped_jsx_identifier_status(&id) {
            Some(JsxIdentifierStatus::Annotatable) => false,
            Some(JsxIdentifierStatus::Unannotatable) => true,
            None => {
                self.fragment_component_identifiers.contains(&id)
                    || ident.sym.as_ref() == "Fragment"
            }
        }
    }

    #[inline]
    fn scoped_jsx_identifier_status(&self, id: &Id) -> Option<JsxIdentifierStatus> {
        self.jsx_bindings.status(id)
    }

    #[inline]
    fn is_global_fragment_identifier(&self, ident: &Ident) -> bool {
        let id = ident.to_id();

        self.fragment_component_identifiers.contains(&id)
            || (self.scoped_jsx_identifier_status(&id).is_none()
                && ident.sym.as_ref() == "Fragment")
    }

    #[inline]
    fn is_global_react_namespace_identifier(&self, ident: &Ident) -> bool {
        let id = ident.to_id();

        self.react_namespace_identifiers.contains(&id)
            || (self.scoped_jsx_identifier_status(&id).is_none() && ident.sym.as_ref() == "React")
    }

    #[inline]
    fn is_global_react_memo_identifier(&self, ident: &Ident) -> bool {
        self.react_memo_identifiers.contains(&ident.to_id())
    }

    #[inline]
    fn is_unannotatable_jsx_element_name(&self, element_name: &JSXElementName) -> bool {
        let JSXElementName::Ident(ident) = element_name else {
            return false;
        };

        self.is_unannotatable_identifier(ident)
    }

    #[inline]
    fn is_react_fragment_element_name(&self, element_name: &JSXElementName) -> bool {
        match element_name {
            JSXElementName::Ident(ident) => self.is_global_fragment_identifier(ident),
            JSXElementName::JSXMemberExpr(member_expr) => matches!(
                &member_expr.obj,
                JSXObject::Ident(obj)
                    if self.is_global_react_namespace_identifier(obj)
                        && member_expr.prop.sym.as_ref() == "Fragment"
            ),
            JSXElementName::JSXNamespacedName(_) => false,
            #[cfg(swc_ast_unknown)]
            _ => false,
        }
    }

    fn expr_may_resolve_to_fragment(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident(ident) => self.is_unannotatable_identifier(ident),
            Expr::Member(member_expr) => {
                let prop_is_fragment = matches!(
                    &member_expr.prop,
                    MemberProp::Ident(prop) if prop.sym.as_ref() == "Fragment"
                );
                prop_is_fragment
                    && matches!(
                        member_expr.obj.as_ref(),
                        Expr::Ident(obj)
                            if self.is_global_react_namespace_identifier(obj)
                    )
            }
            Expr::Cond(cond_expr) => {
                self.expr_may_resolve_to_fragment(&cond_expr.cons)
                    || self.expr_may_resolve_to_fragment(&cond_expr.alt)
            }
            Expr::Paren(paren_expr) => self.expr_may_resolve_to_fragment(&paren_expr.expr),
            Expr::Array(array_expr) => {
                array_expr.elems.iter().flatten().any(|elem| {
                    elem.spread.is_none() && self.expr_may_resolve_to_fragment(&elem.expr)
                })
            }
            _ => false,
        }
    }

    fn register_react_imports(&mut self, import_decl: &ImportDecl) {
        if import_decl.src.value != "react" {
            return;
        }

        for specifier in &import_decl.specifiers {
            match specifier {
                ImportSpecifier::Default(default_import) => {
                    self.react_namespace_identifiers
                        .insert(default_import.local.to_id());
                }
                ImportSpecifier::Namespace(namespace_import) => {
                    self.react_namespace_identifiers
                        .insert(namespace_import.local.to_id());
                }
                ImportSpecifier::Named(named_import) => {
                    let imported_name = match &named_import.imported {
                        Some(ModuleExportName::Ident(ident)) => Some(ident.sym.as_ref()),
                        Some(ModuleExportName::Str(str)) => str.value.as_str(),
                        None => Some(named_import.local.sym.as_ref()),
                        #[cfg(swc_ast_unknown)]
                        Some(_) => None,
                    };

                    match imported_name {
                        Some("Fragment") => {
                            self.fragment_component_identifiers
                                .insert(named_import.local.to_id());
                        }
                        Some("memo") => {
                            self.react_memo_identifiers
                                .insert(named_import.local.to_id());
                        }
                        _ => {}
                    }
                }
                #[cfg(swc_ast_unknown)]
                _ => {}
            }
        }
    }

    fn register_decl_bindings(&mut self, decl: &Decl) {
        match decl {
            Decl::Class(class_decl) => {
                self.jsx_bindings
                    .insert(class_decl.ident.to_id(), JsxIdentifierStatus::Annotatable);
            }
            Decl::Fn(fn_decl) => {
                self.jsx_bindings
                    .insert(fn_decl.ident.to_id(), JsxIdentifierStatus::Annotatable);
            }
            Decl::Var(var_decl) => {
                self.register_var_decl_bindings(var_decl);
            }
            _ => {}
        }
    }

    fn register_var_decl_bindings(&mut self, var_decl: &VarDecl) {
        for declarator in &var_decl.decls {
            self.register_commonjs_import(declarator);
            self.register_var_declarator_binding_with_kind(declarator, var_decl.kind);
        }
    }

    fn register_commonjs_import(&mut self, declarator: &VarDeclarator) {
        let Some(source) = declarator.init.as_deref().and_then(commonjs_require_source) else {
            return;
        };

        match source {
            "react/compiler-runtime" => {
                self.react_compiler_enabled = true;
            }
            "react" => {
                if let Pat::Ident(ident) = &declarator.name {
                    self.react_namespace_identifiers.insert(ident.id.to_id());
                }
            }
            _ => {}
        }
    }

    fn register_var_declarator_binding_with_kind(
        &mut self,
        declarator: &VarDeclarator,
        kind: VarDeclKind,
    ) {
        let status = if declarator
            .init
            .as_deref()
            .is_some_and(|init| self.expr_may_resolve_to_fragment(init))
        {
            JsxIdentifierStatus::Unannotatable
        } else {
            JsxIdentifierStatus::Annotatable
        };
        let is_var = kind == VarDeclKind::Var;

        collect_pat_identifiers_with_status(&declarator.name, status, &mut |ident, status| {
            if is_var {
                self.jsx_bindings.insert_var(ident.to_id(), status);
            } else {
                self.jsx_bindings.insert(ident.to_id(), status);
            }
        });
    }

    fn mark_fragment_assignment(&mut self, assign_expr: &AssignExpr) {
        // React Compiler may hoist a JSX tag into an initially unassigned
        // binding (`T0 = Fragment`) before rendering `<T0>`.
        if assign_expr.op != AssignOp::Assign
            || !self.expr_may_resolve_to_fragment(&assign_expr.right)
        {
            return;
        }

        let AssignTarget::Simple(SimpleAssignTarget::Ident(target)) = &assign_expr.left else {
            return;
        };

        self.jsx_bindings.mark_unannotatable(&target.id.to_id());
    }

    fn visit_var_declarator(&mut self, var_declarator: &mut VarDeclarator) {
        let is_definition =
            matches!(&var_declarator.name, Pat::Ident(_)) && var_declarator.init.is_some();
        let role = if is_definition {
            self.return_root_definitions.next_role()
        } else {
            DefinitionRole::Other
        };
        let component_name = match &var_declarator.name {
            Pat::Ident(ident) if is_component_name(ident.id.sym.as_ref()) => {
                Some(ident.id.sym.clone())
            }
            _ => None,
        };

        var_declarator.name.visit_mut_with(self);

        if role != DefinitionRole::Other {
            if let Some(init) = &mut var_declarator.init {
                self.visit_compiled_definition(init, role);
            }
            return;
        }

        if component_name.as_ref().is_some_and(|component_name| {
            self.should_skip_component_child_traversal(component_name)
        }) {
            return;
        }

        if let Some(init) = &mut var_declarator.init {
            let init = unwrap_component_expr(init);
            match init {
                Expr::Call(call_expr) => {
                    if let Some(component_name) = component_name.clone() {
                        if self.visit_react_memo_component(call_expr, component_name) {
                            return;
                        }
                    }

                    if self.config.experimental_rewrite_emotion_styled {
                        if let (Some(component_name), Some(ref_component_name)) = (
                            component_name.as_ref(),
                            styled_call_component_ref(call_expr, self.styled_import.as_ref()),
                        ) {
                            if self.component_annotation_policy(component_name.as_ref())
                                == ComponentAnnotationPolicy::Normal
                            {
                                transform_styled_call(
                                    call_expr,
                                    ref_component_name,
                                    component_name.clone(),
                                    StyledTransformAttrs {
                                        element_attr_name: self.config.element_attr_name(),
                                        source_file: self.source_file_name.as_ref(),
                                        source_file_attr_ident: &self.source_file_attr_ident,
                                        source_path: self.source_file_path.as_ref(),
                                        source_path_attr_ident: self
                                            .source_path_attr_ident
                                            .as_ref(),
                                    },
                                );
                            }
                        }
                    }
                    init.visit_mut_with(self);
                }
                Expr::Arrow(arrow_func) => {
                    if let Some(component_name) = component_name {
                        self.visit_arrow_as_component(arrow_func, component_name);
                    } else {
                        init.visit_mut_with(self);
                    }
                }
                Expr::Fn(func_expr) => {
                    if let Some(component_name) = component_name {
                        self.visit_function_as_component(&mut func_expr.function, component_name);
                    } else {
                        init.visit_mut_with(self);
                    }
                }
                _ => init.visit_mut_with(self),
            }
        }
    }

    fn visit_var_decl(&mut self, var_decl: &mut VarDecl, register_bindings: bool) {
        if register_bindings {
            self.register_var_decl_bindings(var_decl);
        }

        for declarator in &mut var_decl.decls {
            self.visit_var_declarator(declarator);
        }
    }

    fn visit_predeclared_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Decl(Decl::Var(var_decl)) => {
                self.visit_var_decl(var_decl, false);
            }
            _ => stmt.visit_mut_with(self),
        }
    }

    fn visit_predeclared_module_item(&mut self, item: &mut ModuleItem) {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                match &mut export_decl.decl {
                    Decl::Var(var_decl) => {
                        self.visit_var_decl(var_decl, false);
                    }
                    _ => item.visit_mut_with(self),
                }
            }
            ModuleItem::Stmt(stmt) => {
                self.visit_predeclared_stmt(stmt);
            }
            _ => item.visit_mut_with(self),
        }
    }

    fn register_module_item_bindings(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                self.register_react_imports(import_decl);
                self.react_compiler_enabled |= import_decl.src.value == "react/compiler-runtime";
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                self.register_decl_bindings(&export_decl.decl);
            }
            ModuleItem::Stmt(Stmt::Decl(decl)) => {
                self.register_decl_bindings(decl);
            }
            _ => {}
        }
    }

    fn register_script_stmt_bindings(&mut self, stmt: &Stmt) {
        if let Stmt::Decl(decl) = stmt {
            self.register_decl_bindings(decl);
        }
    }

    fn is_react_memo_call(&self, call_expr: &CallExpr) -> bool {
        let Some(callee) = call_expr.callee.as_expr() else {
            return false;
        };

        match callee.as_ref() {
            Expr::Ident(ident) => self.is_global_react_memo_identifier(ident),
            Expr::Member(member_expr) => {
                matches!(&member_expr.prop, MemberProp::Ident(prop) if prop.sym.as_ref() == "memo")
                    && matches!(
                        member_expr.obj.as_ref(),
                        Expr::Ident(obj) if self.is_global_react_namespace_identifier(obj)
                    )
            }
            _ => false,
        }
    }

    fn visit_react_memo_component(
        &mut self,
        call_expr: &mut CallExpr,
        component_name: Atom,
    ) -> bool {
        if !self.is_react_memo_call(call_expr) {
            return false;
        }

        let Some(component_arg) = call_expr.args.first_mut() else {
            return false;
        };
        if component_arg.spread.is_some() {
            return false;
        }

        match unwrap_component_expr(&mut component_arg.expr) {
            Expr::Arrow(arrow_expr) => {
                self.visit_arrow_as_component(arrow_expr, component_name);
            }
            Expr::Fn(function_expr) => {
                self.visit_function_as_component(&mut function_expr.function, component_name);
            }
            _ => return false,
        }

        for arg in call_expr.args.iter_mut().skip(1) {
            arg.visit_mut_with(self);
        }

        true
    }

    fn process_jsx_element(&mut self, element: &mut JSXElement) {
        // Check if this is a named fragment (Fragment, React.Fragment, or aliases)
        let is_fragment = self.is_react_fragment_element_name(&element.opening.name);

        if is_fragment {
            self.visit_fragment_jsx_children(element);
            return;
        }

        self.add_attributes_to_element(&mut element.opening);

        // The <svg> itself is annotated, nothing inside it is. Its children are still
        // walked so `_temp` helpers it renders are skipped too.
        let prev_in_svg = self.in_svg;
        self.in_svg |= is_svg_element_name(&element.opening.name);
        self.visit_element_jsx_children(element);
        self.in_svg = prev_in_svg;
    }

    fn process_jsx_fragment(&mut self, fragment: &mut JSXFragment) {
        self.visit_fragment_jsx_children(fragment);
    }

    fn visit_fragment_jsx_children<N>(&mut self, node: &mut N)
    where
        N: VisitMutWith<Self>,
    {
        let prev_component = self.current_component_name.clone();
        if self.current_component_name.is_none() {
            self.current_component_name = self.fragment_child_component_name.clone();
        }

        node.visit_mut_children_with(self);
        self.current_component_name = prev_component;
    }

    fn visit_element_jsx_children<N>(&mut self, node: &mut N)
    where
        N: VisitMutWith<Self>,
    {
        let prev_component = self.current_component_name.take();
        let prev_fragment_child_component = self.fragment_child_component_name.clone();

        if self.fragment_child_component_name.is_none() {
            self.fragment_child_component_name = prev_component.clone();
        }

        node.visit_mut_children_with(self);

        self.current_component_name = prev_component;
        self.fragment_child_component_name = prev_fragment_child_component;
    }

    fn add_attributes_to_element(&self, opening_element: &mut JSXOpeningElement) {
        if self.in_svg || self.is_unannotatable_jsx_element_name(&opening_element.name) {
            return;
        }

        let Some(element_name) = get_element_name(&opening_element.name) else {
            return;
        };
        let element_policy = self.component_annotation_policy(&element_name);
        let transparent_owner_component_name =
            if element_policy == ComponentAnnotationPolicy::Transparent {
                self.transparent_owner_component_name(&element_name)
            } else {
                None
            };

        let existing_attrs = attribute_presence(
            opening_element,
            &self.component_attr_ident,
            &self.element_attr_ident,
            &self.source_file_attr_ident,
            self.source_path_attr_ident.as_ref(),
        );
        let Some(element_attrs) = self.element_attrs(
            &element_name,
            element_policy,
            &existing_attrs,
            transparent_owner_component_name.as_ref(),
            !is_noise_element_name(&opening_element.name),
        ) else {
            return;
        };

        match element_attrs.insertion_position {
            AttributeInsertionPosition::Append => {
                opening_element.attrs.extend(element_attrs.attrs);
            }
            AttributeInsertionPosition::BeforeFirstSpread => {
                let insert_at = opening_element
                    .attrs
                    .iter()
                    .position(|attr| matches!(attr, JSXAttrOrSpread::SpreadElement(_)))
                    .unwrap_or(opening_element.attrs.len());
                opening_element
                    .attrs
                    .splice(insert_at..insert_at, element_attrs.attrs);
            }
        }
    }

    /// Owner of JSX rendered at this point, if any. Ignored components lend no name.
    fn render_owner(&self) -> Option<Atom> {
        self.current_component_name
            .as_ref()
            .or(self.fragment_child_component_name.as_ref())
            .or(self.active_callback_owner.as_ref())
            .filter(|owner| {
                self.component_annotation_policy(owner.as_ref())
                    == ComponentAnnotationPolicy::Normal
            })
            .cloned()
    }

    fn visit_function_as_component(&mut self, func: &mut Function, component_name: Atom) {
        if self.should_skip_component_child_traversal(component_name.as_ref()) {
            return;
        }

        let prev_callback_owners = (
            self.callback_render_owner.take(),
            self.active_callback_owner.take(),
        );
        let prev_return_component = self.current_return_component_name.replace(component_name);
        let prev_component = self.current_component_name.take();
        let prev_fragment_child_component = self.fragment_child_component_name.take();
        let prev_return_root_definitions = std::mem::replace(
            &mut self.return_root_definitions,
            if self.react_compiler_enabled {
                return_root_definitions_from_function(func)
            } else {
                ReturnRootDefinitions::default()
            },
        );

        self.jsx_bindings
            .push_function_with(collect_function_param_scope(&func.params));
        func.visit_mut_children_with(self);
        self.jsx_bindings.pop();

        (self.callback_render_owner, self.active_callback_owner) = prev_callback_owners;
        self.current_return_component_name = prev_return_component;
        self.current_component_name = prev_component;
        self.fragment_child_component_name = prev_fragment_child_component;
        debug_assert_eq!(
            self.return_root_definitions.next_definition,
            self.return_root_definitions.definition_count
        );
        self.return_root_definitions = prev_return_root_definitions;
    }

    fn visit_arrow_as_component(&mut self, arrow_expr: &mut ArrowExpr, component_name: Atom) {
        if self.should_skip_component_child_traversal(component_name.as_ref()) {
            return;
        }

        let prev_callback_owners = (
            self.callback_render_owner.take(),
            self.active_callback_owner.take(),
        );
        let prev_return_component = self.current_return_component_name.replace(component_name);
        let prev_component = self.current_component_name.take();
        let prev_fragment_child_component = self.fragment_child_component_name.take();
        let prev_return_root_definitions = std::mem::replace(
            &mut self.return_root_definitions,
            if self.react_compiler_enabled {
                return_root_definitions_from_arrow(arrow_expr)
            } else {
                ReturnRootDefinitions::default()
            },
        );

        self.jsx_bindings
            .push_function_with(collect_pat_list_scope(&arrow_expr.params));
        match arrow_expr.body.as_mut() {
            ArrowFunctionBody::FunctionBody(body) => {
                body.visit_mut_with(self);
            }
            ArrowFunctionBody::Expr(expr) => {
                self.visit_component_return_expr(expr);
            }
            #[cfg(swc_ast_unknown)]
            _ => {}
        }
        self.jsx_bindings.pop();

        (self.callback_render_owner, self.active_callback_owner) = prev_callback_owners;
        self.current_return_component_name = prev_return_component;
        self.current_component_name = prev_component;
        self.fragment_child_component_name = prev_fragment_child_component;
        debug_assert_eq!(
            self.return_root_definitions.next_definition,
            self.return_root_definitions.definition_count
        );
        self.return_root_definitions = prev_return_root_definitions;
    }

    fn visit_component_return_expr(&mut self, expr: &mut Expr) {
        let Some(component_name) = self.current_return_component_name.clone() else {
            self.visit_callback_return_expr(expr);
            return;
        };

        let prev_component = self.current_component_name.replace(component_name);
        let prev_fragment_child_component = self.fragment_child_component_name.take();

        expr.visit_mut_with(self);

        self.current_component_name = prev_component;
        self.fragment_child_component_name = prev_fragment_child_component;
    }

    /// Visits a compiled definition of JSX the component returns or an `<svg>` renders
    fn visit_compiled_definition(&mut self, value: &mut Expr, role: DefinitionRole) {
        match role {
            DefinitionRole::ReturnRoot => self.visit_component_return_expr(value),
            DefinitionRole::SvgContent => {
                let prev_in_svg = std::mem::replace(&mut self.in_svg, true);
                value.visit_mut_with(self);
                self.in_svg = prev_in_svg;
            }
            DefinitionRole::Other => value.visit_mut_with(self),
        }
    }

    /// Visits what a callback returns with the owner its render context lends it.
    /// Outside render callbacks that owner is `None`, so nothing changes.
    fn visit_callback_return_expr(&mut self, expr: &mut Expr) {
        let prev_active_owner = std::mem::replace(
            &mut self.active_callback_owner,
            self.callback_render_owner.clone(),
        );
        expr.visit_mut_with(self);
        self.active_callback_owner = prev_active_owner;
    }
}

impl VisitMut for ReactComponentAnnotateVisitor {
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, module: &mut Module) {
        for item in &module.body {
            self.register_module_item_bindings(item);
        }

        for item in &mut module.body {
            self.visit_predeclared_module_item(item);
        }
    }

    fn visit_mut_script(&mut self, script: &mut Script) {
        for stmt in &script.body {
            self.register_script_stmt_bindings(stmt);
        }

        for stmt in &mut script.body {
            self.visit_predeclared_stmt(stmt);
        }
    }

    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        // Track imports from @emotion/styled (only if enabled)
        if self.config.experimental_rewrite_emotion_styled
            && import_decl.src.value == "@emotion/styled"
        {
            for specifier in &import_decl.specifiers {
                match specifier {
                    // Default import: import styled from '@emotion/styled'
                    ImportSpecifier::Default(default_import) => {
                        self.styled_import = Some(default_import.local.to_id());
                    }
                    // Named import: import { styled } from '@emotion/styled'
                    ImportSpecifier::Named(named_import) => {
                        // Check if the imported name is 'default' or 'styled'
                        let imported_name = match &named_import.imported {
                            Some(ModuleExportName::Ident(ident)) => Some(ident.sym.as_ref()),
                            Some(ModuleExportName::Str(str)) => str.value.as_str(),
                            None => Some(named_import.local.sym.as_ref()),
                            #[cfg(swc_ast_unknown)]
                            Some(_) => None,
                        };

                        if let Some(imported_name) = imported_name {
                            if imported_name == "default" || imported_name == "styled" {
                                self.styled_import = Some(named_import.local.to_id());
                            }
                        }
                    }
                    ImportSpecifier::Namespace(_) => {}
                    #[cfg(swc_ast_unknown)]
                    _ => {}
                }
            }
        }

        import_decl.visit_mut_children_with(self);
    }

    fn visit_mut_fn_decl(&mut self, func_decl: &mut FnDecl) {
        let component_name = func_decl.ident.sym.clone();
        self.jsx_bindings
            .insert(func_decl.ident.to_id(), JsxIdentifierStatus::Annotatable);

        // React Compiler helpers (`_temp`, `_temp2`, ...) are extracted inline
        // callbacks, not components; traverse without attributing their JSX. The
        // compiler emits them after the component that renders them, so transparent
        // callsites inside can still fall back to that component.
        if is_react_compiler_temp(component_name.as_ref()) {
            let owner = self
                .temp_helper_owners
                .get(&component_name)
                .cloned()
                .flatten();
            let prev_active_owner = std::mem::replace(&mut self.active_callback_owner, owner);
            let prev_in_svg = std::mem::replace(
                &mut self.in_svg,
                self.svg_temp_helpers.contains(&component_name),
            );
            self.visit_mut_function(&mut func_decl.function);
            self.active_callback_owner = prev_active_owner;
            self.in_svg = prev_in_svg;
        } else if is_component_name(component_name.as_ref()) {
            self.visit_function_as_component(&mut func_decl.function, component_name);
        } else {
            self.visit_mut_function(&mut func_decl.function);
        }
    }

    fn visit_mut_ident(&mut self, ident: &mut Ident) {
        if !self.react_compiler_enabled || !is_react_compiler_temp(ident.sym.as_ref()) {
            return;
        }
        if self.in_svg {
            self.svg_temp_helpers.insert(ident.sym.clone());
            return;
        }
        let Some(owner) = self.render_owner() else {
            return;
        };

        self.temp_helper_owners
            .entry(ident.sym.clone())
            .and_modify(|existing| {
                if existing.as_ref() != Some(&owner) {
                    *existing = None;
                }
            })
            .or_insert_with(|| Some(owner));
    }

    fn visit_mut_function(&mut self, function: &mut Function) {
        // Only what this callback returns renders for the current owner; its
        // `return` statements pick the owner up again in `visit_component_return_expr`.
        let render_owner = self.render_owner();
        let prev_callback_owners = (
            std::mem::replace(&mut self.callback_render_owner, render_owner),
            self.active_callback_owner.take(),
        );
        let prev_return_component = self.current_return_component_name.take();
        let prev_component = self.current_component_name.take();
        let prev_fragment_child_component = self.fragment_child_component_name.take();
        let prev_return_root_definitions = std::mem::take(&mut self.return_root_definitions);

        self.jsx_bindings
            .push_function_with(collect_function_param_scope(&function.params));
        function.visit_mut_children_with(self);
        self.jsx_bindings.pop();

        (self.callback_render_owner, self.active_callback_owner) = prev_callback_owners;
        self.current_return_component_name = prev_return_component;
        self.current_component_name = prev_component;
        self.fragment_child_component_name = prev_fragment_child_component;
        self.return_root_definitions = prev_return_root_definitions;
    }

    fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
        self.visit_var_decl(var_decl, true);
    }

    fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
        let component_name = class_decl.ident.sym.clone();
        self.jsx_bindings
            .insert(class_decl.ident.to_id(), JsxIdentifierStatus::Annotatable);

        if self.should_skip_component_child_traversal(component_name.as_ref()) {
            return;
        }

        for member in &mut class_decl.class.body {
            match member {
                ClassMember::Method(method) if matches!(&method.key, PropName::Ident(ident) if ident.sym.as_ref() == "render") =>
                {
                    self.visit_function_as_component(&mut method.function, component_name.clone());
                }
                ClassMember::Method(method) => method.visit_mut_children_with(self),
                _ => member.visit_mut_with(self),
            }
        }
    }

    fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
        self.jsx_bindings.push_block();

        for stmt in &block_stmt.stmts {
            self.register_script_stmt_bindings(stmt);
        }

        for stmt in &mut block_stmt.stmts {
            self.visit_predeclared_stmt(stmt);
        }
        self.jsx_bindings.pop();
    }

    fn visit_mut_function_body(&mut self, function_body: &mut FunctionBody) {
        for stmt in &function_body.stmts {
            self.register_script_stmt_bindings(stmt);
        }

        for stmt in &mut function_body.stmts {
            self.visit_predeclared_stmt(stmt);
        }
    }

    fn visit_mut_switch_stmt(&mut self, switch_stmt: &mut SwitchStmt) {
        switch_stmt.discriminant.visit_mut_with(self);
        self.jsx_bindings.push_block();

        for case in &switch_stmt.cases {
            for stmt in &case.cons {
                self.register_script_stmt_bindings(stmt);
            }
        }

        for case in &mut switch_stmt.cases {
            if let Some(test) = &mut case.test {
                test.visit_mut_with(self);
            }

            for stmt in &mut case.cons {
                self.visit_predeclared_stmt(stmt);
            }
        }

        self.jsx_bindings.pop();
    }

    fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
        let render_owner = self.render_owner();
        let prev_callback_owners = (
            std::mem::replace(&mut self.callback_render_owner, render_owner),
            self.active_callback_owner.take(),
        );
        let prev_return_component = self.current_return_component_name.take();
        let prev_component = self.current_component_name.take();
        let prev_fragment_child_component = self.fragment_child_component_name.take();
        let prev_return_root_definitions = std::mem::take(&mut self.return_root_definitions);

        self.jsx_bindings
            .push_function_with(collect_pat_list_scope(&arrow_expr.params));
        arrow_expr.params.visit_mut_with(self);
        match arrow_expr.body.as_mut() {
            // A concise body is the callback's return value
            ArrowFunctionBody::Expr(expr) => self.visit_callback_return_expr(expr),
            body => body.visit_mut_with(self),
        }
        self.jsx_bindings.pop();

        (self.callback_render_owner, self.active_callback_owner) = prev_callback_owners;
        self.current_return_component_name = prev_return_component;
        self.current_component_name = prev_component;
        self.fragment_child_component_name = prev_fragment_child_component;
        self.return_root_definitions = prev_return_root_definitions;
    }

    fn visit_mut_assign_expr(&mut self, assign_expr: &mut AssignExpr) {
        self.mark_fragment_assignment(assign_expr);

        let is_definition = assign_expr.op == AssignOp::Assign
            && matches!(
                &assign_expr.left,
                AssignTarget::Simple(SimpleAssignTarget::Ident(_))
            );
        let role = if is_definition {
            self.return_root_definitions.next_role()
        } else {
            DefinitionRole::Other
        };

        if role == DefinitionRole::Other {
            assign_expr.visit_mut_children_with(self);
        } else {
            self.visit_compiled_definition(&mut assign_expr.right, role);
        }
    }

    fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
        if let Some(arg) = &mut return_stmt.arg {
            self.visit_component_return_expr(arg);
        }
    }

    fn visit_mut_jsx_element(&mut self, jsx_element: &mut JSXElement) {
        self.process_jsx_element(jsx_element);
    }

    fn visit_mut_jsx_fragment(&mut self, jsx_fragment: &mut JSXFragment) {
        self.process_jsx_fragment(jsx_fragment);
    }
}

/// React components are PascalCase. Lowercase functions that return JSX, like
/// `renderRow` or `getIcon`, are helpers whose names would only be DOM noise.
#[inline]
fn is_component_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Matches the module-scope helper names the React Compiler generates when it
/// extracts inline callbacks / render closures (`_temp`, `_temp2`, ...).
#[inline]
fn is_react_compiler_temp(name: &str) -> bool {
    name.strip_prefix("_temp")
        .is_some_and(|rest| rest.bytes().all(|byte| byte.is_ascii_digit()))
}

#[plugin_transform]
pub fn process_transform(
    mut program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    let config = match metadata
        .get_transform_plugin_config()
        .map_or(Ok(PluginConfig::default()), |config| {
            PluginConfig::from_json(&config)
        }) {
        Ok(config) => config,
        Err(error) => {
            // Falling back to the defaults would silently change every attribute
            // name. Fail through SWC's handler, since a panic in wasm loses its
            // message and the host blames an `swc_core` version mismatch instead.
            HANDLER.with(|handler| {
                handler.err(&format!(
                    "swc-plugin-component-annotate: invalid config: {error}"
                ));
            });
            return program;
        }
    };

    // Try to get the actual filename from the metadata context
    let filename = if let Some(filename_str) =
        metadata.get_context(&TransformPluginMetadataContextKind::Filename)
    {
        FileName::Custom(filename_str)
    } else {
        FileName::Custom("unknown".to_string())
    };

    let mut visitor = ReactComponentAnnotateVisitor::new(config, &filename);
    program.visit_mut_with(&mut visitor);
    program
}
