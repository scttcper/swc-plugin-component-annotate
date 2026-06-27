use rustc_hash::FxHashMap;
use swc_core::ecma::ast::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsxIdentifierStatus {
    Annotatable,
    Unannotatable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JsxBindingScopeKind {
    Module,
    Function,
    Block,
}

#[derive(Default)]
pub(crate) struct JsxBindingTracker {
    scopes: Vec<JsxBindingScope>,
}

#[derive(Default)]
struct JsxBindingScope {
    kind: JsxBindingScopeKind,
    bindings: Option<FxHashMap<Id, JsxIdentifierStatus>>,
}

impl JsxBindingTracker {
    pub(crate) fn new() -> Self {
        Self {
            scopes: vec![JsxBindingScope::new(JsxBindingScopeKind::Module)],
        }
    }

    pub(crate) fn push_block(&mut self) {
        self.scopes
            .push(JsxBindingScope::new(JsxBindingScopeKind::Block));
    }

    pub(crate) fn push_function_with(&mut self, bindings: FxHashMap<Id, JsxIdentifierStatus>) {
        self.scopes.push(JsxBindingScope {
            kind: JsxBindingScopeKind::Function,
            bindings: (!bindings.is_empty()).then_some(bindings),
        });
    }

    pub(crate) fn pop(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn insert(&mut self, id: Id, status: JsxIdentifierStatus) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(id, status);
        }
    }

    pub(crate) fn insert_var(&mut self, id: Id, status: JsxIdentifierStatus) {
        if let Some(scope) = self.scopes.iter_mut().rev().find(|scope| {
            matches!(
                scope.kind,
                JsxBindingScopeKind::Function | JsxBindingScopeKind::Module
            )
        }) {
            scope.insert(id, status);
        }
    }

    pub(crate) fn status(&self, id: &Id) -> Option<JsxIdentifierStatus> {
        self.scopes
            .iter()
            .rev()
            .filter_map(|scope| scope.bindings.as_ref())
            .find_map(|scope| scope.get(id).copied())
    }
}

impl JsxBindingScope {
    fn new(kind: JsxBindingScopeKind) -> Self {
        Self {
            kind,
            bindings: None,
        }
    }

    fn insert(&mut self, id: Id, status: JsxIdentifierStatus) {
        self.bindings
            .get_or_insert_with(FxHashMap::default)
            .insert(id, status);
    }
}

impl Default for JsxBindingScopeKind {
    fn default() -> Self {
        Self::Block
    }
}

pub(crate) fn collect_function_param_scope(params: &[Param]) -> FxHashMap<Id, JsxIdentifierStatus> {
    let mut scope = FxHashMap::default();

    for param in params {
        collect_pat_identifiers(&param.pat, &mut scope, JsxIdentifierStatus::Unannotatable);
    }

    scope
}

pub(crate) fn collect_pat_list_scope(params: &[Pat]) -> FxHashMap<Id, JsxIdentifierStatus> {
    let mut scope = FxHashMap::default();

    for param in params {
        collect_pat_identifiers(param, &mut scope, JsxIdentifierStatus::Unannotatable);
    }

    scope
}

fn collect_pat_identifiers(
    pat: &Pat,
    scope: &mut FxHashMap<Id, JsxIdentifierStatus>,
    status: JsxIdentifierStatus,
) {
    collect_pat_identifiers_with_status(pat, status, &mut |ident, status| {
        scope.insert(ident.to_id(), status);
    });
}

pub(crate) fn collect_pat_identifiers_with_status(
    pat: &Pat,
    status: JsxIdentifierStatus,
    add: &mut dyn FnMut(&Ident, JsxIdentifierStatus),
) {
    match pat {
        Pat::Ident(binding_ident) => {
            add(&binding_ident.id, status);
        }
        Pat::Array(array_pat) => {
            for elem in array_pat.elems.iter().flatten() {
                collect_pat_identifiers_with_status(elem, status, add);
            }
        }
        Pat::Object(object_pat) => {
            for prop in &object_pat.props {
                match prop {
                    ObjectPatProp::KeyValue(key_value) => {
                        collect_pat_identifiers_with_status(&key_value.value, status, add);
                    }
                    ObjectPatProp::Assign(assign) => {
                        add(&assign.key.id, status);
                    }
                    ObjectPatProp::Rest(rest) => {
                        collect_pat_identifiers_with_status(&rest.arg, status, add);
                    }
                    #[cfg(swc_ast_unknown)]
                    _ => panic!("unknown object pattern prop"),
                }
            }
        }
        Pat::Rest(rest_pat) => {
            collect_pat_identifiers_with_status(&rest_pat.arg, status, add);
        }
        Pat::Assign(assign_pat) => {
            collect_pat_identifiers_with_status(&assign_pat.left, status, add);
        }
        #[cfg(swc_ast_unknown)]
        Pat::Unknown(..) => panic!("unknown pattern"),
        _ => {}
    }
}
