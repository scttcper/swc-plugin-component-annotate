// The <svg> is annotated like any other element, nothing inside it is
export function IconClose(props) {
    return <svg viewBox="0 0 16 16" {...props} data-component="IconClose" data-source-file="test.jsx">
      <g>
        <path d="M0 0L16 16"/>
      </g>
    </svg>;
}
// Components and callbacks rendered inside an <svg> are skipped too
export function Chart({ points }) {
    return <div data-component="Chart" data-source-file="test.jsx">
      <svg>
        <Bar/>
        {points.map((point)=><Dot key={point}/>)}
      </svg>
      <Legend data-element="Legend" data-source-file="test.jsx"/>
    </div>;
}
// Lowercase and namespaced tags are intrinsic, so they never get an element name
export function Widgets() {
    return <section data-component="Widgets" data-source-file="test.jsx">
      <my-widget/>
      <ns:widget/>
      <math>
        <mi>x</mi>
      </math>
    </section>;
}
// Only <svg> contents are skipped, a bare SVG root is annotated as usual
export function Arrow() {
    return <path d="M0 0" data-component="Arrow" data-source-file="test.jsx"/>;
}
