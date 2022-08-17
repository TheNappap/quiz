
import { relogin } from "./login.js"
import { loadTitle } from "./utils.js"

function onLoad() {
	loadTitle();
	relogin();
}

window.onload = onLoad;