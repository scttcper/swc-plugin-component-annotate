pub mod config;
mod constants;
mod jsx_utils;
pub mod path_utils;

use config::PluginConfig;
use jsx_utils::*;
use path_utils::{extract_absolute_path, extract_filename};
use rustc_hash::FxHashSet;
use swc_core::{
    common::FileName,
    ecma::{
        ast::*,
        visit::{noop_visit_mut_type, VisitMut, VisitMutWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

pub struct ReactComponentAnnotateVisitor {
    config: PluginConfig,
    source_file_name: Option<String>,
    source_file_path: Option<String>,
    current_component_name: Option<String>,
    ignored_elements: FxHashSet<&'static str>,
    ignored_components_set: FxHashSet<String>,
    /// Track the local identifier name for `styled` from @emotion/styled
    styled_import: Option<String>,
}

impl ReactComponentAnnotateVisitor {
    pub fn new(config: PluginConfig, filename: &FileName) -> Self {
        let source_file_name = extract_filename(filename);
        let source_file_path = extract_absolute_path(filename);

        // Pre-compute ignored components set for O(1) lookups
        let ignored_components_set: FxHashSet<String> =
            config.ignored_components.iter().cloned().collect();

        Self {
            config,
            source_file_name,
            source_file_path,
            current_component_name: None,
            ignored_elements: constants::default_ignored_elements(),
            ignored_components_set,
            styled_import: None,
        }
    }

    #[inline]
    pub fn should_ignore_component(&self, component_name: &str) -> bool {
        self.ignored_components_set.contains(component_name)
    }

    #[inline]
    fn should_ignore_element(&self, element_name: &str) -> bool {
        self.ignored_elements.contains(element_name)
    }

    fn process_jsx_element(&mut self, element: &mut JSXElement) {
        // Check if this is a named fragment (Fragment, React.Fragment)
        let is_fragment = is_react_fragment(&element.opening.name);

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
                _ => {}
            }
        }
    }

    fn add_attributes_to_element(&self, opening_element: &mut JSXOpeningElement) {
        let element_name = get_element_name(&opening_element.name);

        // Skip React fragments
        if is_react_fragment(&opening_element.name) {
            return;
        }

        // Check if component should be ignored
        if let Some(ref component_name) = self.current_component_name {
            if self.should_ignore_component(component_name) {
                return;
            }
        }

        // Check if element should be ignored
        if self.should_ignore_component(&element_name) {
            return;
        }

        let is_ignored_html = self.should_ignore_element(&element_name);

        // Add element attribute (for non-HTML elements or when component name differs)
        if !is_ignored_html
            && !has_attribute(opening_element, self.config.element_attr_name())
            && (self.config.component_attr_name() != self.config.element_attr_name()
                || self.current_component_name.is_none())
        {
            opening_element.attrs.push(create_jsx_attr(
                self.config.element_attr_name(),
                &element_name,
            ));
        }

        // Add component attribute (only for root elements)
        if let Some(ref component_name) = self.current_component_name {
            if !has_attribute(opening_element, self.config.component_attr_name()) {
                opening_element.attrs.push(create_jsx_attr(
                    self.config.component_attr_name(),
                    component_name,
                ));
            }
        }

        // Add source file attribute
        if let Some(ref source_file) = self.source_file_name {
            if (self.current_component_name.is_some() || !is_ignored_html)
                && !has_attribute(opening_element, self.config.source_file_attr_name())
            {
                opening_element.attrs.push(create_jsx_attr(
                    self.config.source_file_attr_name(),
                    source_file,
                ));
            }
        }

        // Add source path attribute (only if explicitly configured)
        if self.config.source_path_attr.is_some() {
            if let Some(ref source_path) = self.source_file_path {
                if (self.current_component_name.is_some() || !is_ignored_html)
                    && !has_attribute(opening_element, self.config.source_path_attr_name())
                {
                    opening_element.attrs.push(create_jsx_attr(
                        self.config.source_path_attr_name(),
                        source_path,
                    ));
                }
            }
        }
    }

    fn find_jsx_in_function_body(&mut self, func: &mut Function, component_name: String) {
        if let Some(body) = &mut func.body {
            self.current_component_name = Some(component_name);

            // Look for return statements
            for stmt in &mut body.stmts {
                if let Stmt::Return(return_stmt) = stmt {
                    if let Some(arg) = &mut return_stmt.arg {
                        self.process_return_expression(arg);
                    }
                }
            }

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
        let mut attrs = vec![];

        // Add data-element attribute using the styled component variable name
        attrs.push(create_jsx_attr(
            self.config.element_attr_name(),
            &styled_component_name,
        ));

        // Add data-source-file attribute
        if let Some(ref source_file) = self.source_file_name {
            attrs.push(create_jsx_attr(
                self.config.source_file_attr_name(),
                source_file,
            ));
        }

        // Add data-source-path attribute (only if explicitly configured)
        if self.config.source_path_attr.is_some() {
            if let Some(ref source_path) = self.source_file_path {
                attrs.push(create_jsx_attr(
                    self.config.source_path_attr_name(),
                    source_path,
                ));
            }
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
}

impl VisitMut for ReactComponentAnnotateVisitor {
    noop_visit_mut_type!();

    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        // Track imports from @emotion/styled (only if enabled)
        if self.config.experimental_rewrite_emotion_styled
            && import_decl.src.value.as_ref() == "@emotion/styled"
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
                            Some(ModuleExportName::Ident(ident)) => ident.sym.as_ref(),
                            None => named_import.local.sym.as_ref(),
                            _ => continue,
                        };

                        if imported_name == "default" || imported_name == "styled" {
                            self.styled_import = Some(named_import.local.sym.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        import_decl.visit_mut_children_with(self);
    }

    fn visit_mut_fn_decl(&mut self, func_decl: &mut FnDecl) {
        let component_name = func_decl.ident.sym.to_string();
        self.find_jsx_in_function_body(&mut func_decl.function, component_name);
        func_decl.visit_mut_children_with(self);
    }

    fn visit_mut_var_declarator(&mut self, var_declarator: &mut VarDeclarator) {
        // Handle arrow functions and function expressions assigned to variables
        if let Pat::Ident(ident) = &var_declarator.name {
            let component_name = ident.id.sym.to_string();

            if let Some(init) = &mut var_declarator.init {
                match init.as_mut() {
                    Expr::Call(call_expr) => {
                        // Check if this is a styled(ComponentRef) pattern (only if enabled)
                        if self.config.experimental_rewrite_emotion_styled {
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

                        match arrow_func.body.as_mut() {
                            BlockStmtOrExpr::BlockStmt(block) => {
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
                        }

                        self.current_component_name = None;
                    }
                    Expr::Fn(func_expr) => {
                        self.find_jsx_in_function_body(&mut func_expr.function, component_name);
                    }
                    _ => {}
                }
            }
        }

        var_declarator.visit_mut_children_with(self);
    }

    fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
        let component_name = class_decl.ident.sym.to_string();

        // Look for render method
        for member in &mut class_decl.class.body {
            if let ClassMember::Method(method) = member {
                if let PropName::Ident(ident) = &method.key {
                    if ident.sym.as_ref() == "render" {
                        if let Some(body) = &mut method.function.body {
                            self.current_component_name = Some(component_name.clone());

                            // Look for return statements
                            for stmt in &mut body.stmts {
                                if let Stmt::Return(return_stmt) = stmt {
                                    if let Some(arg) = &mut return_stmt.arg {
                                        self.process_return_expression(arg);
                                    }
                                }
                            }

                            self.current_component_name = None;
                        }
                    }
                }
            }
        }

        class_decl.visit_mut_children_with(self);
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
