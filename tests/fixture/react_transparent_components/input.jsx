import React, {memo} from 'react';
import styled from '@emotion/styled';

function ReplayDetails() {
  return <Flex />;
}

function MetricsToolbar() {
  return <Grid />;
}

function FirstLastSeenSection() {
  return <Stack />;
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

function LayoutStackUsage() {
  return <LayoutStack />;
}

const Stack = memo(function Stack(props) {
  return <Flex {...props} />;
});

const Container = styled(({as: Component = 'div', ...rest}) => {
  return <Component {...rest} />;
}, {
  shouldForwardProp: prop => prop !== 'as',
})``;

const Flex = styled(Container, {
  shouldForwardProp: prop => prop !== 'direction',
})``;

const LayoutStackComponent = styled(({direction = 'column', ...props}) => {
  return (
    <React.Fragment>
      <Flex {...props} direction={direction} />
    </React.Fragment>
  );
});

const LayoutStack = Object.assign(LayoutStackComponent, {
  Separator: styled(props => {
    return <hr {...props} />;
  }),
});

const Grid = styled(Container);
const StyledFlex = styled(Container);

function Button() {
  return <button>Click me</button>;
}
