import { html } from "htm/preact";

export function Checkbox({ id, checked, onChange, children }) {
	return html`
    <div class=${styles.checkboxRow}>
      <input
        type="checkbox"
        id=${id}
        name=${id}
        class=${styles.checkbox}
        checked=${checked}
        onChange=${onChange}
      />
      <label for=${id}>${children}</label>
    </div>
  `;
}

const styles = {
	checkboxRow: "flex items-center gap-2",
	checkbox: "w-4 h-4 text-green-600 bg-gray-100 border-gray-300 rounded",
};
