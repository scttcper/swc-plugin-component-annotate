import React from 'react';
import styled from '@emotion/styled';
function ReplayDetails() {
    return <Flex data-component="ReplayDetails" data-source-file="test.jsx"/>;
}
function MetricsToolbar() {
    return <Grid data-component="MetricsToolbar" data-source-file="test.jsx"/>;
}
function CheckoutSummary() {
    return <Stack data-component="CheckoutSummary" data-source-file="test.jsx">
      <Button data-element="Button" data-source-file="test.jsx"/>
      <StyledFlex data-element="StyledFlex" data-source-file="test.jsx"/>
      <Grid/>
    </Stack>;
}
function Flex(props) {
    return <Container {...props}/>;
}
function Stack(props) {
    return <Flex {...props}/>;
}
function Container({ as: Component = 'div', ...rest }) {
    return <Component {...rest}/>;
}
const Grid = styled(Container);
const StyledFlex = styled((props)=><Container data-element="StyledFlex" data-source-file="test.jsx" {...props}/>);
function Button() {
    return <button data-component="Button" data-source-file="test.jsx">Click me</button>;
}
