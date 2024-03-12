import { html } from "htm/preact";

export function Captcha({ captchaKey, isHidden }) {
	if (isHidden) {
		return null;
	}
	return html`
    <div class="h-[100px] flex items-center justify-center mt-4">
      ${
				captchaKey &&
				html`
        <div class="captcha-container">
          <div class="g-recaptcha" data-sitekey="{{ captcha_key }}"></div>
        </div>
      `
			}
      <div class="flex flex-col items-center justify-center gap-2 hidden">
        <div class="loader"></div>
        <div>Waiting until more tokens are available</div>
      </div>
    </div>
  `;
}
