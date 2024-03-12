import { html } from "htm/preact";
import { Component } from "preact";

export class FuelLogo extends Component {
	render() {
		return html`
      <div class="flex items-center justify-center mb-4">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 500 500"
          style="width: 80px; height: 80px"
          class="s_logo__16erN"
        >
          <g data-name="Fuel logo">
            <g g clip-path="url(#a)" data-name="logo">
              <path
                fill="#00F58C"
                d="M28.85,0C12.92,0,0,12.92,0,28.85V434h359.09c12.15,0,23.81-4.83,32.4-13.42l29.09-29.09
         c8.59-8.59,13.42-20.25,13.42-32.4V0H28.85z"
              />
            </g>
            <path
              d="M283.36,55.8L142.22,196.94c-3.5,3.5-8.25,5.47-13.21,5.47h0c-7.22,0-13.8-4.16-16.89-10.69L57.45,76.11
       c-4.46-9.44,2.42-20.31,12.86-20.31H283.36z"
            />
            <path
              d="M55.8,378.2V240.87c0-7.32,5.94-13.26,13.26-13.26h137.33L55.8,378.2z"
            />
            <path
              d="M217.72,202.41h-45.46l136.8-136.81c6.28-6.28,14.79-9.81,23.67-9.81h45.46l-136.8,136.81
       C235.12,198.88,226.6,202.41,217.72,202.41z"
            />
          </g>
        </svg>
      </div>
    `;
	}
}
