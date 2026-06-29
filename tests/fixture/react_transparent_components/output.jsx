import React, { memo } from 'react';
import styled from '@emotion/styled';
function ReplayDetails() {
    return <Flex data-component="ReplayDetails" data-source-file="test.jsx"/>;
}
function MetricsToolbar() {
    return <Grid data-component="MetricsToolbar" data-source-file="test.jsx"/>;
}
function FirstLastSeenSection() {
    return <Stack data-component="FirstLastSeenSection" data-source-file="test.jsx"/>;
}
function CheckoutSummary() {
    return <Stack data-component="CheckoutSummary" data-source-file="test.jsx">
      <Button data-element="Button" data-source-file="test.jsx"/>
      <StyledFlex data-element="StyledFlex" data-source-file="test.jsx"/>
      <Grid/>
    </Stack>;
}
function LayoutStackUsage() {
    return <LayoutStack data-component="LayoutStackUsage" data-source-file="test.jsx"/>;
}
const Stack = memo(function Stack(props) {
    return <Flex {...props}/>;
});
const Container = styled(({ as: Component = 'div', ...rest })=>{
    return <Component {...rest}/>;
}, {
    shouldForwardProp: (prop)=>prop !== 'as'
})``;
const Flex = styled(Container, {
    shouldForwardProp: (prop)=>prop !== 'direction'
})``;
const LayoutStackComponent = styled(({ direction = 'column', ...props })=>{
    return <React.Fragment>
      <Flex {...props} direction={direction}/>
    </React.Fragment>;
});
const LayoutStack = Object.assign(LayoutStackComponent, {
    Separator: styled((props)=>{
        return <hr {...props}/>;
    })
});
const Grid = styled(Container);
const StyledFlex = styled((props)=><Container data-element="StyledFlex" data-source-file="test.jsx" {...props}/>);
function Button() {
    return <button data-component="Button" data-source-file="test.jsx">Click me</button>;
}
