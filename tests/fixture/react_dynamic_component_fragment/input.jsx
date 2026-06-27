import React, {Fragment, Fragment as ReactFragment} from 'react';
import * as ReactNamespace from 'react';

const Provider = ({children}) => <main>{children}</main>;

const UserProvider = ({children}) => <section>{children}</section>;

export function AppProviders({enabled, children}) {
  const OrganizationProvider = enabled ? UserProvider : Fragment;
  const providers = [OrganizationProvider, UserProvider];

  return providers.reduceRight((acc, Provider) => <Provider>{acc}</Provider>, children);
}

export function DirectProvider({enabled, children}) {
  const Provider = enabled ? UserProvider : Fragment;

  return <Provider>{children}</Provider>;
}

export function AliasedImportProvider({enabled, children}) {
  const Provider = enabled ? UserProvider : ReactFragment;

  return <Provider>{children}</Provider>;
}

export function NamespaceProvider({enabled, children}) {
  const Provider = enabled ? UserProvider : ReactNamespace.Fragment;

  return <Provider>{children}</Provider>;
}

export function DefaultNamespaceProvider({enabled, children}) {
  const Provider = enabled ? UserProvider : React.Fragment;

  return <Provider>{children}</Provider>;
}

export function NamespaceFragmentChildren({children}) {
  return <ReactNamespace.Fragment><span>{children}</span></ReactNamespace.Fragment>;
}

export function LocalAliasDoesNotLeak({children}) {
  return <Provider>{children}</Provider>;
}

export function LocalProviderShadowsFragmentAlias({children}) {
  const Provider = ({children}) => <article>{children}</article>;

  return <Provider>{children}</Provider>;
}

export function LocalFragmentShadowsImport({children}) {
  const Fragment = ({children}) => <article>{children}</article>;

  return <Fragment>{children}</Fragment>;
}

export function LocalNamespaceShadowsImport({children}) {
  const ReactNamespace = {
    Fragment: ({children}) => <article>{children}</article>,
  };

  return <ReactNamespace.Fragment>{children}</ReactNamespace.Fragment>;
}

export function PropSlotProvider({Provider, children}) {
  return <Provider>{children}</Provider>;
}

export function RenamedPropSlotProvider({provider: Provider, children}) {
  return <Provider>{children}</Provider>;
}

export function BlockAliasDoesNotLeak({children}) {
  if (children) {
    const Provider = Fragment;
    Provider;
  }

  return <Provider>{children}</Provider>;
}

export function BareBlockAliasDoesNotLeak({children}) {
  {
    const Provider = Fragment;
    Provider;
  }

  return <Provider>{children}</Provider>;
}

export function BareBlockAliasAppliesInside({children}) {
  {
    const Provider = Fragment;
    return <Provider>{children}</Provider>;
  }
}

export function SwitchAliasDoesNotLeak({kind, children}) {
  switch (kind) {
    case 'fragment':
      const Provider = Fragment;
      Provider;
      break;
  }

  return <Provider>{children}</Provider>;
}

export class ClassProviderAlias extends React.Component {
  render() {
    const Provider = Fragment;

    return <Provider>{this.props.children}</Provider>;
  }
}
