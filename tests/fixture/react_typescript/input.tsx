import * as React from 'react';
import { memo } from 'react';
import type { FC, ReactElement } from 'react';

interface Props {
  label: string;
}

type GenericProps<T> = { value: T };

// Type annotations on the binding, params, and return type
export const TypedArrow: FC<Props> = ({ label }) => <span>{label}</span>;

export const GenericArrow = <T extends unknown>({ value }: GenericProps<T>) => <span>{String(value)}</span>;

export function WithReturnType({ label }: Props): ReactElement {
  return <button>{label}</button>;
}

export class ClassComponent extends React.Component<Props> {
  render() {
    return <div>{this.props.label}</div>;
  }
}

// Parens and TypeScript wrappers around the component definition
export const ParenArrow = (({ label }: Props) => <span>{label}</span>);

export const CastArrow = (({ label }: Props) => <span>{label}</span>) as FC<Props>;

export const SatisfiesArrow = (({ label }: Props) => <span>{label}</span>) satisfies FC<Props>;

export const CastMemo = memo(({ label }: Props) => <span>{label}</span>) as typeof TypedArrow;

export const MemoCastArg = memo((({ label }: Props) => <span>{label}</span>) as FC<Props>);

// TypeScript wrappers around the returned JSX
export function AsReturn(): ReactElement {
  return <div /> as ReactElement;
}
