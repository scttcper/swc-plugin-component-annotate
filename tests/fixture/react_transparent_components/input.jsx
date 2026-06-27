import React from 'react';
import styled from '@emotion/styled';

function ReplayDetails() {
  return <Flex />;
}

function MetricsToolbar() {
  return <Grid />;
}

function CheckoutSummary() {
  return (
    <Stack>
      <Button />
      <StyledFlex />
      <Grid />
    </Stack>
  );
}

function Flex(props) {
  return <Container {...props} />;
}

function Stack(props) {
  return <Flex {...props} />;
}

function Container({as: Component = 'div', ...rest}) {
  return <Component {...rest} />;
}

const Grid = styled(Container);
const StyledFlex = styled(Container);

function Button() {
  return <button>Click me</button>;
}
