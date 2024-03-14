import { render } from "preact";
import { App } from "./app.tsx";

render(
	<App providerUrl={window.__PROVIDER_URL__} />,
	document.getElementById("root")!,
);
