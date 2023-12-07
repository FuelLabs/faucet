let working = false;
const u256_max = BigInt(
  "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
);

onmessage = async function (ev) {
  // If already working, stop
  if (working) {
    working = false;
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
    let buffer = await crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(`${salt}${i}`)
    );
    let hash = Array.from(new Uint8Array(buffer))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");

    var bn = BigInt("0x" + hash);

    if (bn <= target) {
      this.postMessage({
        type: "hash",
        value: { salt, nonce: `${i}`, hash },
      });
    }

    i++;
  }

  this.postMessage({ type: "finish" });
};

function getRandomSalt() {
  // Generate a random salt
  let saltArray = new Uint8Array(32);
  crypto.getRandomValues(saltArray);
  return Array.from(saltArray)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
