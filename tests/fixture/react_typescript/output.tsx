import * as React from 'react';
import { memo } from 'react';
import type { FC, ReactElement } from 'react';
interface Props {
    label: string;
}
type GenericProps<T> = {
    value: T;
};
// Type annotations on the binding, params, and return type
export const TypedArrow: FC<Props> = ({ label })=><span data-component="TypedArrow" data-source-file="test.tsx">{label}</span>;
export const GenericArrow = <T extends unknown>({ value }: GenericProps<T>)=><span data-component="GenericArrow" data-source-file="test.tsx">{String(value)}</span>;
export function WithReturnType({ label }: Props): ReactElement {
    return <button data-component="WithReturnType" data-source-file="test.tsx">{label}</button>;
}
export class ClassComponent extends React.Component<Props> {
    render() {
        return <div data-component="ClassComponent" data-source-file="test.tsx">{this.props.label}</div>;
    }
}
// Parens and TypeScript wrappers around the component definition
export const ParenArrow = ({ label }: Props)=><span data-component="ParenArrow" data-source-file="test.tsx">{label}</span>;
export const CastArrow = ({ label }: Props)=><span data-component="CastArrow" data-source-file="test.tsx">{label}</span> as FC<Props>;
export const SatisfiesArrow = ({ label }: Props)=><span data-component="SatisfiesArrow" data-source-file="test.tsx">{label}</span> satisfies FC<Props>;
export const CastMemo = memo(({ label }: Props)=><span data-component="CastMemo" data-source-file="test.tsx">{label}</span>) as typeof TypedArrow;
export const MemoCastArg = memo(({ label }: Props)=><span data-component="MemoCastArg" data-source-file="test.tsx">{label}</span> as FC<Props>);
// TypeScript wrappers around the returned JSX
export function AsReturn(): ReactElement {
    return <div data-component="AsReturn" data-source-file="test.tsx"/> as ReactElement;
}
