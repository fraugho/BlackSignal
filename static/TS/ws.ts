let sender_id = "";
let user_map: { [key: string]: string } = {};
let ws_id = "";
let username = "";
let current_room = "";

let socket: WebSocket;

enum MessageType {
    Basic = 'Basic',
    Image = 'Image',
    Notification = 'Notification',
    Typing = 'Typing',
    UserRemoval = 'UserRemoval',
    UserAddition = 'UserAddition',
    NewUser = 'NewUser',
    ChangeRoom = 'ChangeRoom',
    UsernameChange = 'UsernameChange',
    CreateRoomChange = 'CreateRoomChange',
    Initialization = 'Initialization',
    Deletion = 'Deletion',
}

interface InitMessage {
    user_id: string;
    ws_id: string;
    username: string;
    user_map: { [key: string]: string };
}

interface BasicMessage {
    content: string;
    sender_id: string;
    message_id: string;
    room_id: string;
    ws_id: string;
    timestamp: number;
}

interface ImageMessage {
    image_url: string;
    sender_id: string;
}

interface NotificationMessage {
    sender_id: string;
}

interface TypingMessage {
    sender_id: string;
}

interface UserRemovalMessage {
    removed_user: string;
    sender_id: string;
}

interface UserAdditionMessage {
    user_id: string;
    username: string;
}

interface NewUserMessage {
    user_id: string;
    username: string;
}

interface ChangeRoomMessage {
    room_id: string;
    sender_id: string;
}

interface UsernameChangeMessage {
    new_username: string;
    sender_id: string;
}

interface CreateRoomChangeMessage {
    room_name: string;
    sender_id: string;
}

interface DeletionMessage {
    sender_id: string;
    message_id: string;
}

type UserMessage =
    | { Basic: BasicMessage }
    | { Image: ImageMessage }
    | { Notification: NotificationMessage }
    | { Typing: TypingMessage }
    | { UserRemoval: UserRemovalMessage }
    | { UserAddition: UserAdditionMessage }
    | { NewUser: NewUserMessage }
    | { ChangeRoom: ChangeRoomMessage }
    | { UsernameChange: UsernameChangeMessage }
    | { CreateRoomChange: CreateRoomChangeMessage }
    | { Initialization: InitMessage }
    | { Deletion: DeletionMessage };


fetch('/get-ip')
    .then(response => response.json())
    .then(data => {
        const server_ip = data.ip;
        console.log('Server IP:', server_ip);
        initializeWebSocket(server_ip);
    })
    .catch(error => console.error('Error fetching server IP:', error));

function initializeWebSocket(server_ip: string): void {
    socket = new WebSocket(`ws://${server_ip}:8080/ws/`);
    socket.onclose = (event: CloseEvent): void => {
        if (!event.wasClean) {
            window.location.href = '/login';
        }
    };

    socket.onmessage = (event: MessageEvent): void => {
        const data = JSON.parse(event.data) as UserMessage;
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

function handleInitialization(init_message: InitMessage) {
    sender_id = init_message.user_id;
    ws_id = init_message.ws_id;
    username = init_message.username;
    user_map = init_message.user_map;
}

function handleBasicMessage(basic_message: BasicMessage) {
    if (basic_message.ws_id === ws_id) {
        return;
    }
    
    const chatContainer = document.getElementById('chat-container')!;
    const messageContainer = document.createElement('div');
    const messageWrapper = document.createElement('div');
    const usernameElement = document.createElement('div');
    const messageElement = document.createElement('div');

    messageContainer.classList.add('message-container');
    messageContainer.setAttribute('data-message-id', basic_message.message_id);
    messageWrapper.classList.add('chat-message');
    messageWrapper.setAttribute('data-message-id', basic_message.message_id);

    usernameElement.textContent = user_map[basic_message.sender_id] !== null ? user_map[basic_message.sender_id] + ':' : 'DeletedAccount:';
    usernameElement.classList.add('username');
    messageElement.textContent = basic_message.content;
    
    messageWrapper.appendChild(usernameElement);
    messageWrapper.appendChild(messageElement);

    if (basic_message.sender_id === sender_id) {
        messageWrapper.classList.add('sent-message');
        messageContainer.classList.add('sent-container');

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.classList.add('delete-checkbox');
        checkbox.style.display = isDeleteMode ? 'inline-block' : 'none';

        messageContainer.appendChild(checkbox);
    } else {
        messageWrapper.classList.add('received-message');
        messageContainer.classList.add('received-container');
    }

    messageContainer.appendChild(messageWrapper);
    chatContainer.appendChild(messageContainer);
    chatContainer.scrollTop = chatContainer.scrollHeight;
}



let isDeleteMode = false; // Keeps track of whether delete mode is active

// Add this after the WebSocket initialization
document.getElementById('toggle-delete-mode')?.addEventListener('click', () => {
    isDeleteMode = !isDeleteMode;
    // Update the checkbox visibility
    const checkboxes = document.querySelectorAll('.delete-checkbox') as NodeListOf<HTMLInputElement>;
    checkboxes.forEach(checkbox => {
        checkbox.style.display = isDeleteMode ? 'block' : 'none';
    });

    // Optionally change the button text to indicate the mode
    const toggleDeleteButton = document.getElementById('toggle-delete-mode') as HTMLButtonElement;
    toggleDeleteButton.textContent = isDeleteMode ? 'Exit Delete Mode' : 'Enter Delete Mode';

    // Show/hide the delete selected messages button
    const deleteSelectedButton = document.getElementById('delete-selected-messages') as HTMLButtonElement;
    deleteSelectedButton.style.display = isDeleteMode ? 'block' : 'none';
});

document.getElementById('delete-selected-messages')?.addEventListener('click', () => {
    const selectedCheckboxes = document.querySelectorAll('.delete-checkbox:checked') as NodeListOf<HTMLInputElement>;
    console.log(`Found ${selectedCheckboxes.length} messages to delete.`);

    selectedCheckboxes.forEach(checkbox => {
        const messageContainer = checkbox.closest('.message-container');
        console.log("1");
        if (messageContainer) {
            console.log("2");
            const messageId = messageContainer.getAttribute('data-message-id');
            if (messageId) {
                console.log("3");
                console.log(`Deleting message with ID: ${messageId}`);
                deleteMessage(messageId);
                console.log(messageContainer);
                messageContainer.remove();
            }
        }
    });

    if (isDeleteMode) {
        document.getElementById('toggle-delete-mode')?.click();
    }
});


function handleImageMessage(imageMessage: ImageMessage) {}
function handleNotificationMessage(notificationMessage: NotificationMessage) {}
function handleTypingMessage(typingMessage: TypingMessage) {}
function handleNewUserMessage(new_user_message: NewUserMessage) {
    user_map[new_user_message.user_id] = new_user_message.username;
}
function handleUserAdditionMessage(user_addition_message: UserAdditionMessage) {}
function handleUserRemovalMessage(userRemovalMessage: UserRemovalMessage) {}
function handleChangeRoomMessage(changeRoomMessage: ChangeRoomMessage) {}
function handleMessageDeletionMessage(message: DeletionMessage) {
    // Extract the message_id from the deletion message
    const messageId = message.message_id;

    // Find the message element in the DOM
    const messageElement = document.querySelector(`[data-message-id="${messageId}"]`);

    // If the element exists, remove it
    if (messageElement) {
        messageElement.remove();
    } else {
        console.log(`Message with ID ${messageId} not found.`);
    }
}
function handleUsernameChangeMessage(username_change_message: UsernameChangeMessage) {
    retroactivelyChangeUsername(user_map[username_change_message.sender_id],username_change_message.new_username);
    user_map[username_change_message.sender_id] = username_change_message.new_username;
    if (sender_id === username_change_message.sender_id){
        username = username_change_message.new_username;
    }
}

function retroactivelyChangeUsername(old_username: string, new_username: string): void {
    const chatContainer = document.getElementById('chat-container');
    if (chatContainer) {
        const messages = chatContainer.getElementsByClassName('chat-message');
    
        for (let i = 0; i < messages.length; i++) {
            const message = messages[i];
            const usernameElements = message.getElementsByClassName('username');
            if (usernameElements.length > 0) {
                const usernameElement = usernameElements[0] as HTMLElement;
                if (usernameElement.textContent?.startsWith(old_username + ':')) {
                    usernameElement.textContent = new_username + ':';
                }
            }
        }
    }
}

function handleCreateRoomChangeMessage(createRoomChangeMessage: CreateRoomChangeMessage) {}

document.getElementById('Message')!.addEventListener('submit', function(event: Event) {
    event.preventDefault();
    sendMessage();
});

function sendMessage(): void {
    const textarea: HTMLTextAreaElement = document.querySelector('textarea[name="message_form"]')!;
    const messageContent: string = textarea.value.trim();
    if (messageContent !== '') {
        const basicMessage: BasicMessage = {
            content: messageContent,
            sender_id: sender_id,
            message_id: "",
            room_id: current_room,
            ws_id: ws_id,
            timestamp: Date.now(),
        };
    
        const wrappedMessage: UserMessage = {
            Basic: basicMessage
        };
    
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(wrappedMessage));
        }
        const chatContainer: HTMLElement = document.getElementById('chat-container')!;
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




const textarea = document.querySelector('textarea[name="message_form"]') as HTMLTextAreaElement;
if (textarea) {
    textarea.addEventListener('keydown', function(event: KeyboardEvent) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
}

function sendUsernameChange(new_username: string): void {
    const usernameChangeMessage: UsernameChangeMessage = {
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
    .then(async response => {
        if (!response.ok) {
            // Directly handle non-ok responses
            const errorData = await response.json(); // Parse the response body to get the error message
            throw new Error(errorData.error || 'Network response was not ok');
        }
        return response.json(); // Parse the response body for ok responses
    })
    .then(data => {
        console.log("Username change successful:", data);
        displayMessage('Username change successful!', 'success'); // Show success message
    })
    .catch(error => {
        console.error('There has been a problem with your fetch operation:', error.message);
        displayMessage(error.message, 'error'); // Show error message
    });    
}

function displayMessage(message: string, type: 'success' | 'error'): void {
    const messageContainer = document.getElementById('message-container');
    if (messageContainer) {
        messageContainer.innerText = message;
        messageContainer.className = '';
        messageContainer.style.opacity = '1';
        messageContainer.style.pointerEvents = 'auto';
        if (type === 'success') {
            messageContainer.classList.add('success-message');
        } else if (type === 'error') {
            messageContainer.classList.add('error-message');
        }
        setTimeout(() => {
            messageContainer.style.opacity = '0';
            messageContainer.style.pointerEvents = 'none';
            setTimeout(() => messageContainer.innerText = '', 500);
        }, 5000);
    }
}

document.getElementById('Username')!.addEventListener('submit', function(event: Event) {
    event.preventDefault();
    const usernameField: HTMLInputElement = document.getElementById('usernameField')! as HTMLInputElement;
    const new_username: string = usernameField.value.trim();
    if (new_username !== '') {
        sendUsernameChange(new_username);
    }
});

document.getElementById('Username')!.addEventListener('keydown', function(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        const usernameField: HTMLInputElement = document.getElementById('usernameField')! as HTMLInputElement;
        const new_username: string = usernameField.value.trim();
        if (new_username !== '') {
            sendUsernameChange(new_username);
        }
    }
});

function deleteMessage(message_id: string): void {
    console.log(`Attempting to delete message with ID: ${message_id}`); // Confirm function is called

    const message: DeletionMessage = {
        sender_id: sender_id,
        message_id: message_id
    };

    const wrapped_message: UserMessage = {
        Deletion: message
    };

    console.log(`WebSocket ready state: ${socket.readyState}`); // Check WebSocket state

    if (socket.readyState === WebSocket.OPEN) {
        console.log(`Sending deletion request for message ID: ${message_id}`, wrapped_message); // Log the message being sent
        socket.send(JSON.stringify(wrapped_message));
    } else {
        console.error("WebSocket is not open. Cannot send message.");
    }
}


  

document.querySelector('input[type="file"][name="image"]')!.addEventListener('change', function(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
        const file = input.files[0];
        const reader = new FileReader();
        reader.onload = function(e) {
            const base64Image = (e.target!.result as string);
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
