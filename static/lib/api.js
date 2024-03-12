export async function dispense(payload, method) {
	const res = await fetch(`/api/dispense?method=${method}`, {
		method: "POST",
		headers: {
			Accept: "application/json",
			"Content-Type": "application/json",
		},
		body: JSON.stringify(payload),
	});

	return res.json();
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
