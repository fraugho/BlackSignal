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
var _a, _b;
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
    MessageType["Deletion"] = "Deletion";
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
            case 'Deletion' in data:
                handleMessageDeletionMessage(data.Deletion);
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
function handleBasicMessage(message) {
    if (message.ws_id === ws_id) {
        const chatContainer = document.getElementById('chat-container');
        const sentContainers = chatContainer.getElementsByClassName('sent-container');
        if (sentContainers.length > 0) {
            const lastSentContainer = sentContainers[sentContainers.length - 1];
            lastSentContainer.setAttribute('data-message-id', message.message_id);
        }
        return;
    }
    const chatContainer = document.getElementById('chat-container');
    const messageContainer = document.createElement('div');
    const messageWrapper = document.createElement('div');
    const usernameElement = document.createElement('div');
    const messageElement = document.createElement('div');
    messageContainer.classList.add('message-container');
    messageContainer.setAttribute('data-message-id', message.message_id);
    messageWrapper.classList.add('chat-message');
    messageWrapper.setAttribute('data-message-id', message.message_id);
    usernameElement.textContent = user_map[message.sender_id] !== null ? user_map[message.sender_id] + ':' : 'DeletedAccount:';
    usernameElement.classList.add('username');
    messageElement.textContent = message.content;
    messageWrapper.appendChild(usernameElement);
    messageWrapper.appendChild(messageElement);
    if (message.sender_id === sender_id) {
        messageWrapper.classList.add('sent-message');
        messageContainer.classList.add('sent-container');
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.classList.add('delete-checkbox');
        checkbox.style.display = is_delete_mode ? 'inline-block' : 'none';
        messageContainer.appendChild(checkbox);
    }
    else {
        messageWrapper.classList.add('received-message');
        messageContainer.classList.add('received-container');
    }
    messageContainer.appendChild(messageWrapper);
    chatContainer.appendChild(messageContainer);
    chatContainer.scrollTop = chatContainer.scrollHeight;
}
let is_delete_mode = false; // Keeps track of whether delete mode is active
// Add this after the WebSocket initialization
(_a = document.getElementById('toggle-delete-mode')) === null || _a === void 0 ? void 0 : _a.addEventListener('click', () => {
    is_delete_mode = !is_delete_mode;
    // Update the checkbox visibility
    const checkboxes = document.querySelectorAll('.delete-checkbox');
    checkboxes.forEach(checkbox => {
        checkbox.style.display = is_delete_mode ? 'block' : 'none';
    });
    // Optionally change the button text to indicate the mode
    const toggleDeleteButton = document.getElementById('toggle-delete-mode');
    toggleDeleteButton.textContent = is_delete_mode ? 'Exit Delete Mode' : 'Enter Delete Mode';
    // Show/hide the delete selected messages button
    const deleteSelectedButton = document.getElementById('delete-selected-messages');
    deleteSelectedButton.style.display = is_delete_mode ? 'block' : 'none';
});
(_b = document.getElementById('delete-selected-messages')) === null || _b === void 0 ? void 0 : _b.addEventListener('click', () => {
    var _a;
    const selectedCheckboxes = document.querySelectorAll('.delete-checkbox:checked');
    selectedCheckboxes.forEach(checkbox => {
        const messageContainer = checkbox.closest('.message-container');
        if (messageContainer) {
            const messageId = messageContainer.getAttribute('data-message-id');
            if (messageId) {
                deleteMessage(messageId);
                messageContainer.remove();
            }
        }
    });
    if (is_delete_mode) {
        (_a = document.getElementById('toggle-delete-mode')) === null || _a === void 0 ? void 0 : _a.click();
    }
});
function handleImageMessage(message) { }
function handleNotificationMessage(message) { }
function handleTypingMessage(typingMessage) { }
function handleNewUserMessage(message) {
    user_map[message.user_id] = message.username;
}
function handleUserAdditionMessage(message) { }
function handleUserRemovalMessage(message) { }
function handleChangeRoomMessage(message) { }
function handleMessageDeletionMessage(message) {
    // Extract the message_id from the deletion message
    const messageId = message.message_id;
    // Find the message element in the DOM
    const messageElement = document.querySelector(`[data-message-id="${messageId}"]`);
    // If the element exists, remove it
    if (messageElement) {
        messageElement.remove();
    }
}
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
        const basic_message = {
            content: messageContent,
            sender_id: sender_id,
            message_id: "",
            room_id: current_room,
            ws_id: ws_id,
            timestamp: Date.now(),
        };
        const wrappedMessage = {
            Basic: basic_message
        };
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(wrappedMessage));
        }
        const chatContainer = document.getElementById('chat-container');
        const messageContainer = document.createElement('div');
        const messageWrapper = document.createElement('div');
        const usernameElement = document.createElement('div');
        const messageElement = document.createElement('div');
        messageContainer.classList.add('message-container', 'sent-container');
        messageWrapper.classList.add('chat-message', 'sent-message');
        usernameElement.textContent = username + ':';
        usernameElement.classList.add('username');
        messageElement.textContent = messageContent;
        messageWrapper.appendChild(usernameElement);
        messageWrapper.appendChild(messageElement);
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.classList.add('delete-checkbox');
        checkbox.style.display = 'none';
        messageContainer.appendChild(checkbox);
        messageContainer.appendChild(messageWrapper);
        chatContainer.appendChild(messageContainer);
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
    const username_field = document.getElementById('username_field');
    const new_username = username_field.value.trim();
    if (new_username !== '') {
        sendUsernameChange(new_username);
    }
});
document.getElementById('Username').addEventListener('keydown', function (event) {
    if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        const username_field = document.getElementById('username_field');
        const new_username = username_field.value.trim();
        if (new_username !== '') {
            sendUsernameChange(new_username);
        }
    }
});
function deleteMessage(message_id) {
    const message = {
        sender_id: sender_id,
        message_id: message_id
    };
    const wrapped_message = {
        Deletion: message
    };
    if (socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(wrapped_message));
    }
}
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
