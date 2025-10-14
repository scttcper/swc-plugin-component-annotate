import React from 'react';
import styled from '@emotion/styled';
// A regular React component
const Button = ({ children, ...props })=>{
    return <button {...props} data-component="Button" data-source-file="test.jsx">{children}</button>;
};
// Styled component using component reference
const StyledButton = styled((props)=><Button data-element="Button" data-source-file="test.jsx" {...props}/>);
// Another regular component
const Card = (props)=>{
    return <div className="card" data-component="Card" data-source-file="test.jsx">
      <h2>{props.title}</h2>
      <p>{props.content}</p>
    </div>;
};
// Styled component using component reference
const StyledCard = styled((props)=><Card data-element="Card" data-source-file="test.jsx" {...props}/>);
// Component that uses the styled components
const MyComponent = ()=>{
    return <div data-component="MyComponent" data-source-file="test.jsx">
      <h1>Styled Components Example</h1>
      <StyledButton data-element="StyledButton" data-source-file="test.jsx">Click me</StyledButton>
      <StyledCard title="My Card" content="Card content" data-element="StyledCard" data-source-file="test.jsx"/>
    </div>;
};
export default MyComponent;
