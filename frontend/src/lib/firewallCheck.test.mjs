import assert from "node:assert/strict";
import { firewallCheckFailure, firewallNotCheckedMessage } from "./firewallCheck.ts";

assert.equal(firewallNotCheckedMessage(null), "Not checked yet.");
assert.match(firewallNotCheckedMessage(true), /firewall is on/);
assert.match(firewallNotCheckedMessage(false), /firewall is off/);

const dismissed = firewallCheckFailure(
	"Error executing command as another user: Request dismissed",
);
assert.equal(dismissed.declined, true, "a dismissed polkit prompt is a decline, not an error");
assert.match(dismissed.message, /Firewall status not checked/);
assert.match(firewallCheckFailure("Not authorized").message, /not checked/i);

const real = firewallCheckFailure("ufw: command not found");
assert.equal(real.declined, false, "a real command failure stays visible");
assert.equal(real.message, "ufw: command not found");
assert.equal(firewallCheckFailure("").declined, false);

console.log("firewallCheck: not checked, declined auth and real errors read differently");
