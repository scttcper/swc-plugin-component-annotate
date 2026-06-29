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
      <Button data-component="CheckoutSummary" data-source-file="test.jsx"/>
      <StyledFlex data-element="StyledFlex" data-source-file="test.jsx"/>
      <Grid data-component="CheckoutSummary" data-source-file="test.jsx"/>
    </Stack>;
}
function LayoutStackUsage() {
    return <LayoutStack data-component="LayoutStackUsage" data-source-file="test.jsx"/>;
}
function MergedItem() {
    return <MergedGroup data-element="MergedGroup" data-component="MergedItem" data-source-file="test.jsx">
      <Grid data-component="MergedItem" data-source-file="test.jsx">
        <Flex data-component="MergedItem" data-source-file="test.jsx">
          <Text data-component="MergedItem" data-source-file="test.jsx">
            <Link data-component="MergedItem" data-source-file="test.jsx">Latest event</Link>
          </Text>
        </Flex>
        <Button aria-label="Show fingerprints" data-component="MergedItem" data-source-file="test.jsx"/>
      </Grid>
    </MergedGroup>;
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
const MergedGroup = styled('div')``;
function Button(props) {
    return <button {...props}>Click me</button>;
}
function Text(props) {
    return <span {...props}/>;
}
function Link(props) {
    return <a {...props}/>;
}
