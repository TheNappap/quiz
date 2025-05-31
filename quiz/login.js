
import { onError } from "./utils.js"
import { createEventSource, fetchLatestEvent } from "./event.js"

function onLogin(username) {
	createEventSource();
	window.localStorage.setItem('Quiz_username', username);
	document.getElementById("login_frame").style.display = "none";
	document.getElementById("user").innerHTML = "Username: "+username;
	fetchLatestEvent();
}

function onRelogin(username) {
	onLogin(username);
}

function onLoginFailed(responseText) {
	onError(responseText)
}

function onReloginFailed() {
	document.getElementById("sub_title").innerHTML = "Choose a username";
	document.getElementById("login_frame").style.display = "table";
}

export function login(){
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4) {
			if (xmlHttp.status == 202) {
				console.log("Logged in: " + xmlHttp.responseText);
				onLogin(xmlHttp.responseText);
			} else {
				onLoginFailed(xmlHttp.responseText);
			}
		}
    }
    xmlHttp.open("POST", "/login");
    xmlHttp.send(document.getElementById("name").value);
}

window.login = login;

export function relogin(){
	var username = window.localStorage.getItem('Quiz_username');
	if (username == null) {
		onReloginFailed();
		return
	}
	
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4) {
			if (xmlHttp.status == 202) {
				onRelogin(xmlHttp.responseText);
			} else {
				onReloginFailed();
			}
		}
    }
    xmlHttp.open("POST", "/relogin");
    xmlHttp.send(username);
}