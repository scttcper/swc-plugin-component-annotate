import React from 'react';
import styled from '@emotion/styled';

// A regular React component
const Button = ({ children, ...props }) => {
  return <button {...props}>{children}</button>;
};

// Styled component using component reference
const StyledButton = styled(Button);

// Another regular component
const Card = (props) => {
  return (
    <div className="card">
      <h2>{props.title}</h2>
      <p>{props.content}</p>
    </div>
  );
};

// Styled component using component reference
const StyledCard = styled(Card);

// Component that uses the styled components
const MyComponent = () => {
  return (
    <div>
      <h1>Styled Components Example</h1>
      <StyledButton>Click me</StyledButton>
      <StyledCard title="My Card" content="Card content" />
    </div>
  );
};

export default MyComponent;
