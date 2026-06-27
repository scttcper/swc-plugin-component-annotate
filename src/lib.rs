pub mod config;
mod constants;
mod jsx_utils;
pub mod path_utils;

use config::PluginConfig;
use jsx_utils::*;
use path_utils::{extract_absolute_path, extract_filename};
use rustc_hash::{FxHashMap, FxHashSet};
use swc_core::{
    common::{FileName, DUMMY_SP},
    ecma::{
        ast::*,
        atoms::Atom,
        visit::{noop_visit_mut_type, VisitMut, VisitMutWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum JsxIdentifierStatus {
    Annotatable,
    Unannotatable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ComponentAnnotationPolicy {
    Normal,
    Ignored,
    Transparent,
}

pub struct ReactComponentAnnotateVisitor {
    config: PluginConfig,
    source_file_name: Option<Str>,
    source_file_path: Option<Str>,
    current_component_name: Option<String>,
    ignored_elements: &'static FxHashSet<&'static str>,
    ignored_components_set: FxHashSet<String>,
    transparent_components_set: FxHashSet<String>,
    // JSX identifiers that may render React.Fragment cannot receive custom props.
    jsx_identifier_scopes: Vec<FxHashMap<Atom, JsxIdentifierStatus>>,
    fragment_component_identifiers: FxHashSet<Atom>,
    react_namespace_identifiers: FxHashSet<Atom>,
    component_attr_ident: IdentName,
    element_attr_ident: IdentName,
    source_file_attr_ident: IdentName,
    source_path_attr_ident: Option<IdentName>,
    /// Track the local identifier name for `styled` from @emotion/styled
    styled_import: Option<String>,
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
        let mut fragment_component_identifiers = FxHashSet::default();
        fragment_component_identifiers.insert("Fragment".into());
        let mut react_namespace_identifiers = FxHashSet::default();
        react_namespace_identifiers.insert("React".into());

        Self {
            component_attr_ident,
            config,
            element_attr_ident,
            ignored_elements: constants::default_ignored_elements(),
            ignored_components_set,
            transparent_components_set,
            jsx_identifier_scopes: Vec::new(),
            fragment_component_identifiers,
            react_namespace_identifiers,
            source_file_name,
            source_file_attr_ident,
            source_file_path,
            source_path_attr_ident,
            current_component_name: None,
            styled_import: None,
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
        self.component_annotation_policy(component_name) == ComponentAnnotationPolicy::Transparent
    }

    #[inline]
    fn should_ignore_element(&self, element_name: &str) -> bool {
        self.ignored_elements.contains(element_name)
    }

    #[inline]
    fn is_unannotatable_identifier(&self, ident: &Atom) -> bool {
        match self.scoped_jsx_identifier_status(ident) {
            Some(JsxIdentifierStatus::Annotatable) => false,
            Some(JsxIdentifierStatus::Unannotatable) => true,
            None => self.fragment_component_identifiers.contains(ident),
        }
    }

    #[inline]
    fn scoped_jsx_identifier_status(&self, ident: &Atom) -> Option<JsxIdentifierStatus> {
        self.jsx_identifier_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(ident).copied())
    }

    #[inline]
    fn is_global_fragment_identifier(&self, ident: &Atom) -> bool {
        self.scoped_jsx_identifier_status(ident).is_none()
            && self.fragment_component_identifiers.contains(ident)
    }

    #[inline]
    fn is_global_react_namespace_identifier(&self, ident: &Atom) -> bool {
        self.scoped_jsx_identifier_status(ident).is_none()
            && self.react_namespace_identifiers.contains(ident)
    }

    #[inline]
    fn is_unannotatable_jsx_element_name(&self, element_name: &JSXElementName) -> bool {
        let JSXElementName::Ident(ident) = element_name else {
            return false;
        };

        self.is_unannotatable_identifier(&ident.sym)
    }

    #[inline]
    fn is_react_fragment_element_name(&self, element_name: &JSXElementName) -> bool {
        match element_name {
            JSXElementName::Ident(ident) => self.is_global_fragment_identifier(&ident.sym),
            JSXElementName::JSXMemberExpr(member_expr) => matches!(
                &member_expr.obj,
                JSXObject::Ident(obj)
                    if self.is_global_react_namespace_identifier(&obj.sym)
                        && member_expr.prop.sym.as_ref() == "Fragment"
            ),
            JSXElementName::JSXNamespacedName(_) => false,
            #[cfg(swc_ast_unknown)]
            _ => panic!("unknown jsx element name"),
        }
    }

    fn expr_may_resolve_to_fragment(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident(ident) => self.is_unannotatable_identifier(&ident.sym),
            Expr::Member(member_expr) => {
                let prop_is_fragment = matches!(
                    &member_expr.prop,
                    MemberProp::Ident(prop) if prop.sym.as_ref() == "Fragment"
                );
                prop_is_fragment
                    && matches!(
                        member_expr.obj.as_ref(),
                        Expr::Ident(obj)
                            if self.is_global_react_namespace_identifier(&obj.sym)
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
            #[cfg(swc_ast_unknown)]
            Expr::Unknown(..) => panic!("unknown expr"),
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
                        .insert(default_import.local.sym.clone());
                }
                ImportSpecifier::Namespace(namespace_import) => {
                    self.react_namespace_identifiers
                        .insert(namespace_import.local.sym.clone());
                }
                ImportSpecifier::Named(named_import) => {
                    let imported_name = match &named_import.imported {
                        Some(ModuleExportName::Ident(ident)) => Some(ident.sym.as_ref()),
                        Some(ModuleExportName::Str(str)) => str.value.as_str(),
                        None => Some(named_import.local.sym.as_ref()),
                        #[cfg(swc_ast_unknown)]
                        Some(_) => panic!("unknown module export name"),
                    };

                    if imported_name == Some("Fragment") {
                        self.fragment_component_identifiers
                            .insert(named_import.local.sym.clone());
                    }
                }
                #[cfg(swc_ast_unknown)]
                _ => panic!("unknown import specifier"),
            }
        }
    }

    fn register_jsx_identifier(&mut self, identifier: Atom, status: JsxIdentifierStatus) {
        let should_insert = status == JsxIdentifierStatus::Unannotatable
            || self.should_track_annotatable_identifier(&identifier);
        if !should_insert {
            return;
        }

        if let Some(scope) = self.jsx_identifier_scopes.last_mut() {
            scope.insert(identifier, status);
        } else if status == JsxIdentifierStatus::Unannotatable {
            self.fragment_component_identifiers.insert(identifier);
        }
    }

    #[inline]
    fn should_track_annotatable_identifier(&self, identifier: &Atom) -> bool {
        self.scoped_jsx_identifier_status(identifier).is_some()
            || self.fragment_component_identifiers.contains(identifier)
            || self.react_namespace_identifiers.contains(identifier)
    }

    fn process_jsx_element(&mut self, element: &mut JSXElement) {
        // Check if this is a named fragment (Fragment, React.Fragment, or aliases)
        let is_fragment = self.is_react_fragment_element_name(&element.opening.name);

        if !is_fragment {
            self.add_attributes_to_element(&mut element.opening);
        }

        // Process children - fragments are transparent containers
        for child in &mut element.children {
            match child {
                JSXElementChild::JSXElement(jsx_element) => {
                    if is_fragment {
                        // Fragment children are processed without clearing component name
                        jsx_element.visit_mut_with(self);
                    } else {
                        // Non-fragment children don't get component name, only element name
                        let prev_component = self.current_component_name.take();
                        jsx_element.visit_mut_with(self);
                        self.current_component_name = prev_component;
                    }
                }
                JSXElementChild::JSXFragment(jsx_fragment) => {
                    // Fragments are always transparent containers
                    jsx_fragment.visit_mut_with(self);
                }
                #[cfg(swc_ast_unknown)]
                JSXElementChild::Unknown(..) => panic!("unknown jsx element child"),
                _ => {}
            }
        }
    }

    fn process_jsx_fragment(&mut self, fragment: &mut JSXFragment) {
        // Fragments are transparent containers - just process children
        for child in &mut fragment.children {
            match child {
                JSXElementChild::JSXElement(jsx_element) => {
                    jsx_element.visit_mut_with(self);
                }
                JSXElementChild::JSXFragment(jsx_fragment) => {
                    jsx_fragment.visit_mut_with(self);
                }
                #[cfg(swc_ast_unknown)]
                JSXElementChild::Unknown(..) => panic!("unknown jsx element child"),
                _ => {}
            }
        }
    }

    fn add_attributes_to_element(&self, opening_element: &mut JSXOpeningElement) {
        if self.is_unannotatable_jsx_element_name(&opening_element.name) {
            return;
        }

        let element_name = get_element_name(&opening_element.name);

        // Check if component should be ignored
        if let Some(ref component_name) = self.current_component_name {
            if self.should_skip_component_return(component_name) {
                return;
            }
        }

        if self.should_ignore_component(&element_name) {
            return;
        }

        let is_ignored_html = self.should_ignore_element(&element_name);
        let is_transparent_element = self.should_treat_component_as_transparent(&element_name);
        let can_annotate_element = !is_ignored_html && !is_transparent_element;
        let has_current_component = self.current_component_name.is_some();
        let add_element_attr = can_annotate_element
            && !has_attribute(opening_element, self.config.element_attr_name())
            && (self.config.component_attr_name() != self.config.element_attr_name()
                || !has_current_component);
        let add_component_attr = has_current_component
            && !has_attribute(opening_element, self.config.component_attr_name());
        let add_source_file_attr = self.source_file_name.is_some()
            && (has_current_component || can_annotate_element)
            && !has_attribute(opening_element, self.config.source_file_attr_name());
        let add_source_path_attr = self.source_file_path.is_some()
            && self.source_path_attr_ident.is_some()
            && (has_current_component || can_annotate_element)
            && !has_attribute(opening_element, self.config.source_path_attr_name());

        let attr_count = usize::from(add_element_attr)
            + usize::from(add_component_attr)
            + usize::from(add_source_file_attr)
            + usize::from(add_source_path_attr);

        if attr_count > 0 {
            opening_element.attrs.reserve(attr_count);
        }

        if add_element_attr {
            opening_element.attrs.push(create_jsx_attr_with_ident(
                &self.element_attr_ident,
                &element_name,
            ));
        }

        if add_component_attr {
            if let Some(ref component_name) = self.current_component_name {
                opening_element.attrs.push(create_jsx_attr_with_ident(
                    &self.component_attr_ident,
                    component_name,
                ));
            }
        }

        if add_source_file_attr {
            if let Some(ref source_file) = self.source_file_name {
                opening_element
                    .attrs
                    .push(create_jsx_attr_with_ident_and_str(
                        &self.source_file_attr_ident,
                        source_file,
                    ));
            }
        }

        if add_source_path_attr {
            if let (Some(ref source_path), Some(ref source_path_attr_ident)) =
                (&self.source_file_path, &self.source_path_attr_ident)
            {
                opening_element
                    .attrs
                    .push(create_jsx_attr_with_ident_and_str(
                        source_path_attr_ident,
                        source_path,
                    ));
            }
        }
    }

    fn find_jsx_in_function_body(&mut self, func: &mut Function, component_name: String) {
        if let Some(body) = &mut func.body {
            self.current_component_name = Some(component_name);
            self.jsx_identifier_scopes
                .push(collect_function_param_scope(&func.params));

            // Register aliases before processing returns so `<Provider />` can be skipped
            // when `Provider` may resolve to Fragment.
            for stmt in &body.stmts {
                self.register_fragment_aliases_in_stmt(stmt);
            }

            // Look for return statements
            for stmt in &mut body.stmts {
                if let Stmt::Return(return_stmt) = stmt {
                    if let Some(arg) = &mut return_stmt.arg {
                        self.process_return_expression(arg);
                    }
                }
            }

            self.jsx_identifier_scopes.pop();
            self.current_component_name = None;
        }
    }

    fn process_return_expression(&mut self, expr: &mut Expr) {
        match expr {
            Expr::JSXElement(jsx_element) => {
                jsx_element.visit_mut_with(self);
            }
            Expr::JSXFragment(jsx_fragment) => {
                jsx_fragment.visit_mut_with(self);
            }
            Expr::Cond(cond_expr) => {
                // Handle ternary expressions
                self.process_return_expression(&mut cond_expr.cons);
                self.process_return_expression(&mut cond_expr.alt);
            }
            Expr::Paren(paren_expr) => {
                self.process_return_expression(&mut paren_expr.expr);
            }
            #[cfg(swc_ast_unknown)]
            Expr::Unknown(..) => panic!("unknown expr"),
            _ => {}
        }
    }

    /// Check if a call expression matches styled(ComponentRef) pattern
    fn is_styled_call_with_component_ref(&self, call_expr: &CallExpr) -> Option<String> {
        // Check if we have a tracked styled import
        let styled_name = self.styled_import.as_ref()?;

        // Check if the callee is the styled identifier
        let callee_name = match call_expr.callee.as_expr() {
            Some(expr) => match expr.as_ref() {
                Expr::Ident(ident) => ident.sym.as_ref(),
                #[cfg(swc_ast_unknown)]
                Expr::Unknown(..) => panic!("unknown expr"),
                _ => return None,
            },
            _ => return None,
        };

        if callee_name != styled_name {
            return None;
        }

        // Check if the first argument is an identifier (component reference)
        if let Some(ExprOrSpread { spread: None, expr }) = call_expr.args.first() {
            if let Expr::Ident(ident) = expr.as_ref() {
                return Some(ident.sym.to_string());
            }
        }

        None
    }

    /// Transform styled(ComponentRef) to styled(props => <ComponentRef data-element="..." {...props} />)
    fn transform_styled_call(
        &self,
        call_expr: &mut CallExpr,
        ref_component_name: String,
        styled_component_name: String,
    ) {
        use swc_core::common::{SyntaxContext, DUMMY_SP};

        // Create the props parameter: props
        let props_param = Pat::Ident(BindingIdent {
            id: Ident::new("props".into(), DUMMY_SP, SyntaxContext::empty()),
            type_ann: None,
        });

        // Build attributes in order: data attributes first, then spread
        let mut attrs = Vec::with_capacity(
            2 + usize::from(self.source_file_name.is_some())
                + usize::from(
                    self.source_path_attr_ident.is_some() && self.source_file_path.is_some(),
                ),
        );

        // Add data-element attribute using the styled component variable name
        attrs.push(create_jsx_attr(
            self.config.element_attr_name(),
            &styled_component_name,
        ));

        // Add data-source-file attribute
        if let Some(ref source_file) = self.source_file_name {
            attrs.push(create_jsx_attr_with_ident_and_str(
                &self.source_file_attr_ident,
                source_file,
            ));
        }

        // Add data-source-path attribute (only if explicitly configured)
        if let (Some(ref source_path), Some(ref source_path_attr_ident)) =
            (&self.source_file_path, &self.source_path_attr_ident)
        {
            attrs.push(create_jsx_attr_with_ident_and_str(
                source_path_attr_ident,
                source_path,
            ));
        }

        // Add spread attribute AFTER data attributes: {...props}
        attrs.push(JSXAttrOrSpread::SpreadElement(SpreadElement {
            dot3_token: DUMMY_SP,
            expr: Box::new(Expr::Ident(Ident::new(
                "props".into(),
                DUMMY_SP,
                SyntaxContext::empty(),
            ))),
        }));

        // Create JSX element: <ComponentRef data-element="..." data-source-file="..." {...props} />
        let jsx_element = JSXElement {
            span: DUMMY_SP,
            opening: JSXOpeningElement {
                name: JSXElementName::Ident(Ident::new(
                    ref_component_name.into(),
                    DUMMY_SP,
                    SyntaxContext::empty(),
                )),
                span: DUMMY_SP,
                attrs,
                self_closing: true,
                type_args: None,
            },
            children: vec![],
            closing: None,
        };

        // Create arrow function: props => <ComponentRef data-element="..." {...props} />
        let arrow_func = ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: vec![props_param],
            body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::JSXElement(Box::new(
                jsx_element,
            ))))),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        };

        // Replace the first argument with the arrow function
        call_expr.args[0] = ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Arrow(arrow_func)),
        };
    }

    fn register_fragment_aliases_in_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Decl(Decl::Var(var_decl)) => {
                self.register_fragment_aliases_in_var_decl(var_decl);
            }
            #[cfg(swc_ast_unknown)]
            Stmt::Unknown(..) => panic!("unknown statement"),
            _ => {}
        }
    }

    fn register_fragment_aliases_in_var_decl(&mut self, var_decl: &VarDecl) {
        for declarator in &var_decl.decls {
            let Pat::Ident(ident) = &declarator.name else {
                continue;
            };

            let Some(init) = &declarator.init else {
                continue;
            };

            if self.expr_may_resolve_to_fragment(init) {
                self.register_jsx_identifier(
                    ident.id.sym.clone(),
                    JsxIdentifierStatus::Unannotatable,
                );
            } else {
                self.register_jsx_identifier(
                    ident.id.sym.clone(),
                    JsxIdentifierStatus::Annotatable,
                );
            }
        }
    }
}

fn collect_function_param_scope(params: &[Param]) -> FxHashMap<Atom, JsxIdentifierStatus> {
    let mut scope = FxHashMap::default();

    for param in params {
        collect_pat_identifiers(&param.pat, &mut scope);
    }

    scope
}

fn collect_pat_list_scope(params: &[Pat]) -> FxHashMap<Atom, JsxIdentifierStatus> {
    let mut scope = FxHashMap::default();

    for param in params {
        collect_pat_identifiers(param, &mut scope);
    }

    scope
}

fn collect_pat_identifiers(pat: &Pat, scope: &mut FxHashMap<Atom, JsxIdentifierStatus>) {
    match pat {
        Pat::Ident(binding_ident) => {
            scope.insert(
                binding_ident.id.sym.clone(),
                JsxIdentifierStatus::Unannotatable,
            );
        }
        Pat::Array(array_pat) => {
            for elem in array_pat.elems.iter().flatten() {
                collect_pat_identifiers(elem, scope);
            }
        }
        Pat::Object(object_pat) => {
            for prop in &object_pat.props {
                match prop {
                    ObjectPatProp::KeyValue(key_value) => {
                        collect_pat_identifiers(&key_value.value, scope);
                    }
                    ObjectPatProp::Assign(assign) => {
                        scope.insert(
                            assign.key.id.sym.clone(),
                            JsxIdentifierStatus::Unannotatable,
                        );
                    }
                    ObjectPatProp::Rest(rest) => {
                        collect_pat_identifiers(&rest.arg, scope);
                    }
                    #[cfg(swc_ast_unknown)]
                    _ => panic!("unknown object pattern prop"),
                }
            }
        }
        Pat::Rest(rest_pat) => {
            collect_pat_identifiers(&rest_pat.arg, scope);
        }
        Pat::Assign(assign_pat) => {
            collect_pat_identifiers(&assign_pat.left, scope);
        }
        #[cfg(swc_ast_unknown)]
        Pat::Unknown(..) => panic!("unknown pattern"),
        _ => {}
    }
}

impl VisitMut for ReactComponentAnnotateVisitor {
    noop_visit_mut_type!();

    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        self.register_react_imports(import_decl);

        // Track imports from @emotion/styled (only if enabled)
        if self.config.experimental_rewrite_emotion_styled
            && import_decl.src.value == "@emotion/styled"
        {
            for specifier in &import_decl.specifiers {
                match specifier {
                    // Default import: import styled from '@emotion/styled'
                    ImportSpecifier::Default(default_import) => {
                        self.styled_import = Some(default_import.local.sym.to_string());
                    }
                    // Named import: import { styled } from '@emotion/styled'
                    ImportSpecifier::Named(named_import) => {
                        // Check if the imported name is 'default' or 'styled'
                        let imported_name = match &named_import.imported {
                            Some(ModuleExportName::Ident(ident)) => Some(ident.sym.as_ref()),
                            Some(ModuleExportName::Str(str)) => str.value.as_str(),
                            None => Some(named_import.local.sym.as_ref()),
                            #[cfg(swc_ast_unknown)]
                            Some(_) => panic!("unknown module export name"),
                        };

                        if let Some(imported_name) = imported_name {
                            if imported_name == "default" || imported_name == "styled" {
                                self.styled_import = Some(named_import.local.sym.to_string());
                            }
                        }
                    }
                    ImportSpecifier::Namespace(_) => {}
                    #[cfg(swc_ast_unknown)]
                    _ => panic!("unknown import specifier"),
                }
            }
        }

        import_decl.visit_mut_children_with(self);
    }

    fn visit_mut_fn_decl(&mut self, func_decl: &mut FnDecl) {
        let component_name = func_decl.ident.sym.to_string();
        let should_skip_children = self.should_skip_component_child_traversal(&component_name);
        self.find_jsx_in_function_body(&mut func_decl.function, component_name);
        if !should_skip_children {
            func_decl.visit_mut_children_with(self);
        }
    }

    fn visit_mut_function(&mut self, function: &mut Function) {
        self.jsx_identifier_scopes
            .push(collect_function_param_scope(&function.params));
        function.visit_mut_children_with(self);
        self.jsx_identifier_scopes.pop();
    }

    fn visit_mut_var_declarator(&mut self, var_declarator: &mut VarDeclarator) {
        // Handle arrow functions and function expressions assigned to variables
        let mut should_skip_children = false;

        if let Pat::Ident(ident) = &var_declarator.name {
            let component_name = ident.id.sym.to_string();
            let component_policy = self.component_annotation_policy(&component_name);
            should_skip_children = component_policy == ComponentAnnotationPolicy::Transparent;

            if let Some(init) = &mut var_declarator.init {
                if self.expr_may_resolve_to_fragment(init) {
                    self.register_jsx_identifier(
                        ident.id.sym.clone(),
                        JsxIdentifierStatus::Unannotatable,
                    );
                } else {
                    self.register_jsx_identifier(
                        ident.id.sym.clone(),
                        JsxIdentifierStatus::Annotatable,
                    );
                }

                match init.as_mut() {
                    Expr::Call(call_expr) => {
                        // Check if this is a styled(ComponentRef) pattern (only if enabled)
                        if self.config.experimental_rewrite_emotion_styled
                            && component_policy == ComponentAnnotationPolicy::Normal
                        {
                            if let Some(ref_component_name) =
                                self.is_styled_call_with_component_ref(call_expr)
                            {
                                // Transform styled(ComponentRef) to styled(props => <ComponentRef {...props} />)
                                // Use the styled component variable name (e.g., StyledButton) as data-element
                                self.transform_styled_call(
                                    call_expr,
                                    ref_component_name,
                                    component_name.clone(),
                                );
                            }
                        }
                    }
                    Expr::Arrow(arrow_func) => {
                        self.current_component_name = Some(component_name);
                        self.jsx_identifier_scopes
                            .push(collect_pat_list_scope(&arrow_func.params));

                        match arrow_func.body.as_mut() {
                            BlockStmtOrExpr::BlockStmt(block) => {
                                for stmt in &block.stmts {
                                    self.register_fragment_aliases_in_stmt(stmt);
                                }

                                // Look for return statements in block
                                for stmt in &mut block.stmts {
                                    if let Stmt::Return(return_stmt) = stmt {
                                        if let Some(arg) = &mut return_stmt.arg {
                                            self.process_return_expression(arg);
                                        }
                                    }
                                }
                            }
                            BlockStmtOrExpr::Expr(expr) => {
                                // Direct expression return
                                self.process_return_expression(expr);
                            }
                            #[cfg(swc_ast_unknown)]
                            _ => panic!("unknown block stmt or expr"),
                        }

                        self.jsx_identifier_scopes.pop();
                        self.current_component_name = None;
                    }
                    Expr::Fn(func_expr) => {
                        self.find_jsx_in_function_body(&mut func_expr.function, component_name);
                    }
                    #[cfg(swc_ast_unknown)]
                    Expr::Unknown(..) => panic!("unknown expr"),
                    _ => {}
                }
            }
        }

        if !should_skip_children {
            var_declarator.visit_mut_children_with(self);
        }
    }

    fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
        let component_name = class_decl.ident.sym.to_string();
        let should_skip_children = self.should_skip_component_child_traversal(&component_name);

        // Look for render method
        for member in &mut class_decl.class.body {
            match member {
                ClassMember::Method(method) => match &method.key {
                    PropName::Ident(ident) => {
                        if ident.sym.as_ref() == "render" {
                            if let Some(body) = &mut method.function.body {
                                self.current_component_name = Some(component_name.clone());
                                self.jsx_identifier_scopes
                                    .push(collect_function_param_scope(&method.function.params));

                                for stmt in &body.stmts {
                                    self.register_fragment_aliases_in_stmt(stmt);
                                }

                                // Look for return statements
                                for stmt in &mut body.stmts {
                                    if let Stmt::Return(return_stmt) = stmt {
                                        if let Some(arg) = &mut return_stmt.arg {
                                            self.process_return_expression(arg);
                                        }
                                    }
                                }

                                self.jsx_identifier_scopes.pop();
                                self.current_component_name = None;
                            }
                        }
                    }
                    #[cfg(swc_ast_unknown)]
                    PropName::Unknown(..) => panic!("unknown prop name"),
                    _ => {}
                },
                #[cfg(swc_ast_unknown)]
                ClassMember::Unknown(..) => panic!("unknown class member"),
                _ => {}
            }
        }

        if !should_skip_children {
            class_decl.visit_mut_children_with(self);
        }
    }

    fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
        self.jsx_identifier_scopes.push(FxHashMap::default());
        block_stmt.visit_mut_children_with(self);
        self.jsx_identifier_scopes.pop();
    }

    fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
        self.jsx_identifier_scopes
            .push(collect_pat_list_scope(&arrow_expr.params));
        arrow_expr.visit_mut_children_with(self);
        self.jsx_identifier_scopes.pop();
    }

    fn visit_mut_jsx_element(&mut self, jsx_element: &mut JSXElement) {
        self.process_jsx_element(jsx_element);
    }

    fn visit_mut_jsx_fragment(&mut self, jsx_fragment: &mut JSXFragment) {
        self.process_jsx_fragment(jsx_fragment);
    }
}

// Export for testing
pub fn extract_filename_for_test(filename: &FileName) -> Option<String> {
    extract_filename(filename)
}

#[plugin_transform]
pub fn process_transform(
    mut program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    let config = if let Some(config_str) = metadata.get_transform_plugin_config() {
        serde_json::from_str::<PluginConfig>(&config_str).unwrap_or_default()
    } else {
        PluginConfig::default()
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
