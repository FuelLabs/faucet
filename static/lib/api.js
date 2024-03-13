export async function dispense(payload, method) {
	const response = await fetch(`/api/dispense?method=${method}`, {
		method: "POST",
		headers: {
			Accept: "application/json",
			"Content-Type": "application/json",
		},
		body: JSON.stringify(payload),
	});
	return response.json();
}

export async function getSession(payload) {
	const response = await fetch("/api/session", {
		method: "POST",
		headers: {
			Accept: "application/json",
			"Content-Type": "application/json",
		},
		body: JSON.stringify(payload),
	});

	return response.json();
}

export async function validateSession(sessions) {
	const response = await fetch("/api/session/validate", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ value: sessions[0]?.id }),
	});
	return response.json();
}

export async function removeSession() {
	const response = await fetch("/api/session/remove", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({}),
	});
	return response.json();
}
