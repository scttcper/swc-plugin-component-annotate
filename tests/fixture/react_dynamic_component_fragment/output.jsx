import React, { Fragment, Fragment as ReactFragment } from 'react';
import * as ReactNamespace from 'react';
const Provider = ({ children })=><main data-component="Provider" data-source-file="test.jsx">{children}</main>;
const UserProvider = ({ children })=><section data-component="UserProvider" data-source-file="test.jsx">{children}</section>;
export function AppProviders({ enabled, children }) {
    const OrganizationProvider = enabled ? UserProvider : Fragment;
    const providers = [
        OrganizationProvider,
        UserProvider
    ];
    return providers.reduceRight((acc, Provider)=><Provider>{acc}</Provider>, children);
}
export function DirectProvider({ enabled, children }) {
    const Provider = enabled ? UserProvider : Fragment;
    return <Provider>{children}</Provider>;
}
export function AliasedImportProvider({ enabled, children }) {
    const Provider = enabled ? UserProvider : ReactFragment;
    return <Provider>{children}</Provider>;
}
export function NamespaceProvider({ enabled, children }) {
    const Provider = enabled ? UserProvider : ReactNamespace.Fragment;
    return <Provider>{children}</Provider>;
}
export function DefaultNamespaceProvider({ enabled, children }) {
    const Provider = enabled ? UserProvider : React.Fragment;
    return <Provider>{children}</Provider>;
}
export function NamespaceFragmentChildren({ children }) {
    return <ReactNamespace.Fragment><span data-component="NamespaceFragmentChildren" data-source-file="test.jsx">{children}</span></ReactNamespace.Fragment>;
}
export function LocalAliasDoesNotLeak({ children }) {
    return <Provider data-element="Provider" data-component="LocalAliasDoesNotLeak" data-source-file="test.jsx">{children}</Provider>;
}
export function LocalProviderShadowsFragmentAlias({ children }) {
    const Provider = ({ children })=><article data-component="Provider" data-source-file="test.jsx">{children}</article>;
    return <Provider data-element="Provider" data-component="LocalProviderShadowsFragmentAlias" data-source-file="test.jsx">{children}</Provider>;
}
export function LocalFragmentShadowsImport({ children }) {
    const Fragment = ({ children })=><article data-component="Fragment" data-source-file="test.jsx">{children}</article>;
    return <Fragment data-element="Fragment" data-component="LocalFragmentShadowsImport" data-source-file="test.jsx">{children}</Fragment>;
}
export function LocalFragmentShadowsImportBeforeDeclaration({ children }) {
    return <Fragment data-element="Fragment" data-component="LocalFragmentShadowsImportBeforeDeclaration" data-source-file="test.jsx">{children}</Fragment>;
    function Fragment({ children }) {
        return <article data-component="Fragment" data-source-file="test.jsx">{children}</article>;
    }
}
export function LocalNamespaceShadowsImport({ children }) {
    const ReactNamespace = {
        Fragment: ({ children })=><article>{children}</article>
    };
    return <ReactNamespace.Fragment data-element="ReactNamespace.Fragment" data-component="LocalNamespaceShadowsImport" data-source-file="test.jsx">{children}</ReactNamespace.Fragment>;
}
export function PropSlotProvider({ Provider, children }) {
    return <Provider>{children}</Provider>;
}
export function RenamedPropSlotProvider({ provider: Provider, children }) {
    return <Provider>{children}</Provider>;
}
export function BlockAliasDoesNotLeak({ children }) {
    if (children) {
        const Provider = Fragment;
        Provider;
    }
    return <Provider data-element="Provider" data-component="BlockAliasDoesNotLeak" data-source-file="test.jsx">{children}</Provider>;
}
export function BareBlockAliasDoesNotLeak({ children }) {
    {
        const Provider = Fragment;
        Provider;
    }
    return <Provider data-element="Provider" data-component="BareBlockAliasDoesNotLeak" data-source-file="test.jsx">{children}</Provider>;
}
export function BareBlockAliasAppliesInside({ children }) {
    {
        const Provider = Fragment;
        return <Provider>{children}</Provider>;
    }
}
export function SwitchAliasDoesNotLeak({ kind, children }) {
    switch(kind){
        case 'fragment':
            const Provider = Fragment;
            Provider;
            break;
    }
    return <Provider data-element="Provider" data-component="SwitchAliasDoesNotLeak" data-source-file="test.jsx">{children}</Provider>;
}
export class ClassProviderAlias extends React.Component {
    render() {
        const Provider = Fragment;
        return <Provider>{this.props.children}</Provider>;
    }
}
