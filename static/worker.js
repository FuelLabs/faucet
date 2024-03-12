let working = false;
const u256_max = BigInt(
	"0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
);

onmessage = async function (ev) {
	// If already working, stop
	if (working) {
		console.log("worker: stopping");
		working = false;
		this.postMessage({ type: "stopped" });
		return;
	}

	// Sanitize input
	if (!ev || !ev.data) return;

	const difficultyLevel = BigInt(ev.data.difficultyLevel);
	const target = u256_max >> difficultyLevel;
	const { salt } = ev.data;

	working = true;

	let i = 0;

	console.log("Working", difficultyLevel, salt);

	while (working) {
		const buffer = await crypto.subtle.digest(
			"SHA-256",
			new TextEncoder().encode(`${salt}${i}`),
		);
		const hash = Array.from(new Uint8Array(buffer))
			.map((b) => b.toString(16).padStart(2, "0"))
			.join("");

		console.log(hash);
		const bn = BigInt(`0x${hash}`);

		if (bn <= target) {
			console.log("found hash", hash);
			working = false;
			this.postMessage({
				type: "hash",
				value: { salt, nonce: `${i}`, hash },
			});
		} else {
			i += 1;
		}
	}
};

function getRandomSalt() {
	// Generate a random salt
	const saltArray = new Uint8Array(32);
	crypto.getRandomValues(saltArray);
	return Array.from(saltArray)
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("");
}
