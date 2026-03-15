use std::borrow::Cow;
use swc_core::ecma::ast::*;

/// Check if a JSX element is a React Fragment
#[inline]
pub fn is_react_fragment(element: &JSXElementName) -> bool {
    match element {
        JSXElementName::Ident(ident) => ident.sym.as_ref() == "Fragment",
        JSXElementName::JSXMemberExpr(member_expr) => matches!(
            &member_expr.obj,
            JSXObject::Ident(obj)
                if obj.sym.as_ref() == "React" && member_expr.prop.sym.as_ref() == "Fragment"
        ),
        JSXElementName::JSXNamespacedName(_) => false,
        #[cfg(swc_ast_unknown)]
        _ => panic!("unknown jsx element name"),
    }
}

/// Extract the element name from a JSX element
#[inline]
pub fn get_element_name(element: &JSXElementName) -> Cow<str> {
    match element {
        JSXElementName::Ident(ident) => Cow::Borrowed(ident.sym.as_ref()),
        JSXElementName::JSXMemberExpr(member_expr) => {
            Cow::Owned(get_member_expression_name(member_expr))
        }
        JSXElementName::JSXNamespacedName(namespaced) => {
            Cow::Owned(format!("{}:{}", namespaced.ns.sym, namespaced.name.sym))
        }
        #[cfg(swc_ast_unknown)]
        _ => panic!("unknown jsx element name"),
    }
}

/// Recursively build the name for member expressions (e.g., "Components.UI.Button")
fn get_member_expression_name(member_expr: &JSXMemberExpr) -> String {
    fn member_expression_name_len(member_expr: &JSXMemberExpr) -> usize {
        let obj_len = match &member_expr.obj {
            JSXObject::Ident(ident) => ident.sym.len(),
            JSXObject::JSXMemberExpr(nested_member) => member_expression_name_len(nested_member),
            #[cfg(swc_ast_unknown)]
            _ => panic!("unknown jsx object"),
        };

        obj_len + 1 + member_expr.prop.sym.len()
    }

    fn push_member_expression_name(target: &mut String, member_expr: &JSXMemberExpr) {
        match &member_expr.obj {
            JSXObject::Ident(ident) => target.push_str(ident.sym.as_ref()),
            JSXObject::JSXMemberExpr(nested_member) => {
                push_member_expression_name(target, nested_member);
            }
            #[cfg(swc_ast_unknown)]
            _ => panic!("unknown jsx object"),
        }

        target.push('.');
        target.push_str(member_expr.prop.sym.as_ref());
    }

    let mut output = String::with_capacity(member_expression_name_len(member_expr));
    push_member_expression_name(&mut output, member_expr);
    output
}

/// Check if a JSX element already has an attribute with the given name
#[inline]
pub fn has_attribute(element: &JSXOpeningElement, attr_name: &str) -> bool {
    element.attrs.iter().any(|attr| {
        matches!(attr, JSXAttrOrSpread::JSXAttr(jsx_attr)
            if matches!(&jsx_attr.name, JSXAttrName::Ident(ident)
                if ident.sym.as_ref() == attr_name))
    })
}

/// Create a JSX attribute with a string value
#[inline]
pub fn create_jsx_attr(name: &str, value: &str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(IdentName::new(name.into(), Default::default())),
        value: Some(JSXAttrValue::Str(Str {
            span: Default::default(),
            value: value.into(),
            raw: None,
        })),
    })
}

#[inline]
pub fn create_jsx_attr_with_ident(name: &IdentName, value: &str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(name.clone()),
        value: Some(JSXAttrValue::Str(Str {
            span: Default::default(),
            value: value.into(),
            raw: None,
        })),
    })
}

#[inline]
pub fn create_jsx_attr_with_ident_and_str(name: &IdentName, value: &Str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(name.clone()),
        value: Some(JSXAttrValue::Str(value.clone())),
    })
}
