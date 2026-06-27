use crate::jsx_utils::{create_jsx_attr, create_jsx_attr_with_ident_and_str};
use swc_core::{
    common::DUMMY_SP,
    ecma::{ast::*, atoms::Atom},
};

pub(crate) fn styled_call_component_ref(
    call_expr: &CallExpr,
    styled_id: Option<&Id>,
) -> Option<Atom> {
    let styled_id = styled_id?;

    let callee_id = match call_expr.callee.as_expr() {
        Some(expr) => match expr.as_ref() {
            Expr::Ident(ident) => ident.to_id(),
            #[cfg(swc_ast_unknown)]
            Expr::Unknown(..) => panic!("unknown expr"),
            _ => return None,
        },
        _ => return None,
    };

    if &callee_id != styled_id {
        return None;
    }

    if let Some(ExprOrSpread { spread: None, expr }) = call_expr.args.first() {
        if let Expr::Ident(ident) = expr.as_ref() {
            return Some(ident.sym.clone());
        }
    }

    None
}

pub(crate) struct StyledTransformAttrs<'a> {
    pub(crate) element_attr_name: &'a str,
    pub(crate) source_file: Option<&'a Str>,
    pub(crate) source_file_attr_ident: &'a IdentName,
    pub(crate) source_path: Option<&'a Str>,
    pub(crate) source_path_attr_ident: Option<&'a IdentName>,
}

pub(crate) fn transform_styled_call(
    call_expr: &mut CallExpr,
    ref_component_name: Atom,
    styled_component_name: Atom,
    attrs_config: StyledTransformAttrs<'_>,
) {
    let props_param = Pat::Ident(BindingIdent {
        id: Ident::new_no_ctxt("props".into(), DUMMY_SP),
        type_ann: None,
    });

    let mut attrs = Vec::with_capacity(
        2 + usize::from(attrs_config.source_file.is_some())
            + usize::from(
                attrs_config.source_path_attr_ident.is_some() && attrs_config.source_path.is_some(),
            ),
    );

    attrs.push(create_jsx_attr(
        attrs_config.element_attr_name,
        styled_component_name.as_ref(),
    ));

    if let Some(source_file) = attrs_config.source_file {
        attrs.push(create_jsx_attr_with_ident_and_str(
            attrs_config.source_file_attr_ident,
            source_file,
        ));
    }

    if let (Some(source_path), Some(source_path_attr_ident)) = (
        attrs_config.source_path,
        attrs_config.source_path_attr_ident,
    ) {
        attrs.push(create_jsx_attr_with_ident_and_str(
            source_path_attr_ident,
            source_path,
        ));
    }

    attrs.push(JSXAttrOrSpread::SpreadElement(SpreadElement {
        dot3_token: DUMMY_SP,
        expr: Box::new(Expr::Ident(Ident::new_no_ctxt("props".into(), DUMMY_SP))),
    }));

    let jsx_element = JSXElement {
        span: DUMMY_SP,
        opening: JSXOpeningElement {
            name: JSXElementName::Ident(Ident::new_no_ctxt(ref_component_name, DUMMY_SP)),
            span: DUMMY_SP,
            attrs,
            self_closing: true,
            type_args: None,
        },
        children: vec![],
        closing: None,
    };

    let arrow_func = ArrowExpr {
        span: DUMMY_SP,
        ctxt: Default::default(),
        params: vec![props_param],
        body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::JSXElement(Box::new(
            jsx_element,
        ))))),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
    };

    call_expr.args[0] = ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Arrow(arrow_func)),
    };
}
