"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
let sender_id = "";
let user_map = {};
let ws_id = "";
let username = "";
let current_room = "";
let socket;
var MessageType;
(function (MessageType) {
    MessageType["Basic"] = "Basic";
    MessageType["Image"] = "Image";
    MessageType["Notification"] = "Notification";
    MessageType["Typing"] = "Typing";
    MessageType["UserRemoval"] = "UserRemoval";
    MessageType["UserAddition"] = "UserAddition";
    MessageType["NewUser"] = "NewUser";
    MessageType["ChangeRoom"] = "ChangeRoom";
    MessageType["UsernameChange"] = "UsernameChange";
    MessageType["CreateRoomChange"] = "CreateRoomChange";
    MessageType["Initialization"] = "Initialization";
    MessageType["DeleteMessage"] = "DeleteMessage";
})(MessageType || (MessageType = {}));
fetch('/get-ip')
    .then(response => response.json())
    .then(data => {
    const server_ip = data.ip;
    console.log('Server IP:', server_ip);
    initializeWebSocket(server_ip);
})
    .catch(error => console.error('Error fetching server IP:', error));
function initializeWebSocket(server_ip) {
    socket = new WebSocket(`ws://${server_ip}:8080/ws/`);
    socket.onclose = (event) => {
        if (!event.wasClean) {
            window.location.href = '/login';
        }
    };
    socket.onmessage = (event) => {
        const data = JSON.parse(event.data);
        switch (true) {
            case 'Initialization' in data:
                handleInitialization(data.Initialization);
                break;
            case 'Basic' in data:
                handleBasicMessage(data.Basic);
                break;
            case 'Image' in data:
                handleImageMessage(data.Image);
                break;
            case 'Notification' in data:
                handleNotificationMessage(data.Notification);
                break;
            case 'Typing' in data:
                handleTypingMessage(data.Typing);
                break;
            case 'DeleteMessage' in data:
                handleMessageDeletionMessage(data.DeleteMessage);
                break;
            case 'NewUser' in data:
                handleNewUserMessage(data.NewUser);
                break;
            case 'UserAddition' in data:
                handleUserAdditionMessage(data.UserAddition);
                break;
            case 'UserRemoval' in data:
                handleUserRemovalMessage(data.UserRemoval);
                break;
            case 'ChangeRoom' in data:
                handleChangeRoomMessage(data.ChangeRoom);
                break;
            case 'UsernameChange' in data:
                handleUsernameChangeMessage(data.UsernameChange);
                break;
            case 'CreateRoomChange' in data:
                handleCreateRoomChangeMessage(data.CreateRoomChange);
                break;
            default:
                console.error("Unknown message type received");
        }
    };
}
function handleInitialization(init_message) {
    sender_id = init_message.user_id;
    ws_id = init_message.ws_id;
    username = init_message.username;
    user_map = init_message.user_map;
}
function handleBasicMessage(basic_message) {
    if (basic_message.ws_id === ws_id) {
        return;
    }
    const chatContainer = document.getElementById('chat-container');
    const messageWrapper = document.createElement('div');
    const usernameElement = document.createElement('div');
    const messageElement = document.createElement('div');
    if (user_map[basic_message.sender_id] === null) {
        usernameElement.textContent = 'DeletedAccount:';
    }
    else {
        usernameElement.textContent = user_map[basic_message.sender_id] + ':';
    }
    usernameElement.classList.add('username');
    messageElement.textContent = basic_message.content;
    messageWrapper.appendChild(usernameElement);
    messageWrapper.appendChild(messageElement);
    messageWrapper.classList.add('chat-message');
    if (basic_message.sender_id === sender_id) {
        messageWrapper.classList.add('sent-message');
    }
    else {
        messageWrapper.classList.add('received-message');
    }
    chatContainer.appendChild(messageWrapper);
    chatContainer.scrollTop = chatContainer.scrollHeight;
}
function handleImageMessage(imageMessage) { }
function handleNotificationMessage(notificationMessage) { }
function handleTypingMessage(typingMessage) { }
function handleNewUserMessage(new_user_message) {
    user_map[new_user_message.user_id] = new_user_message.username;
}
function handleUserAdditionMessage(user_addition_message) { }
function handleUserRemovalMessage(userRemovalMessage) { }
function handleChangeRoomMessage(changeRoomMessage) { }
function handleMessageDeletionMessage(message) { }
function handleUsernameChangeMessage(username_change_message) {
    retroactivelyChangeUsername(user_map[username_change_message.sender_id], username_change_message.new_username);
    user_map[username_change_message.sender_id] = username_change_message.new_username;
    if (sender_id === username_change_message.sender_id) {
        username = username_change_message.new_username;
    }
}
function retroactivelyChangeUsername(old_username, new_username) {
    var _a;
    const chatContainer = document.getElementById('chat-container');
    if (chatContainer) {
        const messages = chatContainer.getElementsByClassName('chat-message');
        for (let i = 0; i < messages.length; i++) {
            const message = messages[i];
            const usernameElements = message.getElementsByClassName('username');
            if (usernameElements.length > 0) {
                const usernameElement = usernameElements[0];
                if ((_a = usernameElement.textContent) === null || _a === void 0 ? void 0 : _a.startsWith(old_username + ':')) {
                    usernameElement.textContent = new_username + ':';
                }
            }
        }
    }
}
function handleCreateRoomChangeMessage(createRoomChangeMessage) { }
document.getElementById('Message').addEventListener('submit', function (event) {
    event.preventDefault();
    sendMessage();
});
function sendMessage() {
    const textarea = document.querySelector('textarea[name="message_form"]');
    const messageContent = textarea.value.trim();
    if (messageContent !== '') {
        const basicMessage = {
            content: messageContent,
            sender_id: sender_id,
            message_id: "",
            room_id: current_room,
            ws_id: ws_id,
            timestamp: Date.now(),
        };
        const wrappedMessage = {
            Basic: basicMessage
        };
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(wrappedMessage));
        }
        const chatContainer = document.getElementById('chat-container');
        const messageWrapper = document.createElement('div');
        const usernameElement = document.createElement('div');
        const messageElement = document.createElement('div');
        usernameElement.textContent = (username) + ':';
        usernameElement.classList.add('username');
        messageElement.textContent = textarea.value.trim();
        messageWrapper.appendChild(usernameElement);
        messageWrapper.appendChild(messageElement);
        messageWrapper.classList.add('chat-message');
        messageWrapper.classList.add('sent-message');
        chatContainer.appendChild(messageWrapper);
        chatContainer.scrollTop = chatContainer.scrollHeight;
        textarea.value = '';
    }
}
const textarea = document.querySelector('textarea[name="message_form"]');
if (textarea) {
    textarea.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
}
function sendUsernameChange(new_username) {
    const usernameChangeMessage = {
        new_username: new_username,
        sender_id: sender_id,
    };
    fetch('/change_username', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({ UsernameChange: usernameChangeMessage })
    })
        .then((response) => __awaiter(this, void 0, void 0, function* () {
        if (!response.ok) {
            // Directly handle non-ok responses
            const errorData = yield response.json(); // Parse the response body to get the error message
            throw new Error(errorData.error || 'Network response was not ok');
        }
        return response.json(); // Parse the response body for ok responses
    }))
        .then(data => {
        console.log("Username change successful:", data);
        displayMessage('Username change successful!', 'success'); // Show success message
    })
        .catch(error => {
        console.error('There has been a problem with your fetch operation:', error.message);
        displayMessage(error.message, 'error'); // Show error message
    });
}
function displayMessage(message, type) {
    const messageContainer = document.getElementById('message-container');
    if (messageContainer) {
        messageContainer.innerText = message;
        messageContainer.className = '';
        messageContainer.style.opacity = '1';
        messageContainer.style.pointerEvents = 'auto';
        if (type === 'success') {
            messageContainer.classList.add('success-message');
        }
        else if (type === 'error') {
            messageContainer.classList.add('error-message');
        }
        setTimeout(() => {
            messageContainer.style.opacity = '0';
            messageContainer.style.pointerEvents = 'none';
            setTimeout(() => messageContainer.innerText = '', 500);
        }, 5000);
    }
}
document.getElementById('Username').addEventListener('submit', function (event) {
    event.preventDefault();
    const usernameField = document.getElementById('usernameField');
    const new_username = usernameField.value.trim();
    if (new_username !== '') {
        sendUsernameChange(new_username);
    }
});
document.getElementById('Username').addEventListener('keydown', function (event) {
    if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        const usernameField = document.getElementById('usernameField');
        const new_username = usernameField.value.trim();
        if (new_username !== '') {
            sendUsernameChange(new_username);
        }
    }
});
document.querySelector('input[type="file"][name="image"]').addEventListener('change', function (event) {
    const input = event.target;
    if (input.files && input.files.length > 0) {
        const file = input.files[0];
        const reader = new FileReader();
        reader.onload = function (e) {
            const base64Image = e.target.result;
            fetch('/upload', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    filename: file.name,
                    data: base64Image.split(',')[1]
                })
            }).then(response => {
                if (!response.ok) {
                    throw new Error('Network response was not ok');
                }
                return response.json();
            }).then(data => {
            }).catch(error => {
                console.error('There has been a problem with your fetch operation:', error);
            });
        };
        reader.readAsDataURL(file);
    }
});
