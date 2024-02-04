let sender_id: string = "";
let user_map: { [key: string]: string } = {};
let ws_id: string = "";
let username: string = "";
let current_room: string = "";

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
    | { Initialization: InitMessage };

fetch('/get-ip')
    .then(response => response.json())
    .then(data => {
        const server_ip: string = data.ip;
        console.log('Server IP:', server_ip);
        initializeWebSocket(server_ip);
    })
    .catch(error => console.error('Error fetching server IP:', error));

function initializeWebSocket(server_ip: string): void {
    socket = new WebSocket(`ws://${server_ip}:8080/ws/`);
    socket.onclose = function(event: CloseEvent): void {
        if (!event.wasClean) {
            window.location.href = '/login';
        }
    };

    socket.onmessage = function(event: MessageEvent): void {
        const data = JSON.parse(event.data) as UserMessage;
        if ('Initialization' in data) {
            handleInitialization(data.Initialization);
        } else if ('Basic' in data) {
            handleBasicMessage(data.Basic);
        } else if ('Image' in data) {
            handleImageMessage(data.Image);
        } else if ('Notification' in data) {
            handleNotificationMessage(data.Notification);
        } else if ('Typing' in data) {
            handleTypingMessage(data.Typing);
        } else if ('NewUser' in data) {
            handleNewUserMessage(data.NewUser);
        } else if ('UserAddition' in data) {
            handleUserAdditionMessage(data.UserAddition);
        } else if ('UserRemoval' in data) {
            handleUserRemovalMessage(data.UserRemoval);
        } else if ('ChangeRoom' in data) {
            handleChangeRoomMessage(data.ChangeRoom);
        } else if ('UsernameChange' in data) {
            handleUsernameChangeMessage(data.UsernameChange);
        } else if ('CreateRoomChange' in data) {
            handleCreateRoomChangeMessage(data.CreateRoomChange);
        } else {
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
    const messageWrapper = document.createElement('div');
    const usernameElement = document.createElement('div');
    const messageElement = document.createElement('div');

    usernameElement.textContent = user_map[basic_message.sender_id] + ':';
    usernameElement.classList.add('username');
    messageElement.textContent = basic_message.content;

    messageWrapper.appendChild(usernameElement);
    messageWrapper.appendChild(messageElement);
    messageWrapper.classList.add('chat-message');

    if (basic_message.sender_id === sender_id) {
        messageWrapper.classList.add('sent-message');
    } else {
        messageWrapper.classList.add('received-message');
    }

    chatContainer.appendChild(messageWrapper);
    chatContainer.scrollTop = chatContainer.scrollHeight;
}
function handleImageMessage(imageMessage: ImageMessage) {}
function handleNotificationMessage(notificationMessage: NotificationMessage) {}
function handleTypingMessage(typingMessage: TypingMessage) {}
function handleNewUserMessage(new_user_message: NewUserMessage) {
    user_map[new_user_message.user_id] = new_user_message.username;
}
function handleUserAdditionMessage(user_addition_message: UserAdditionMessage) {}
function handleUserRemovalMessage(userRemovalMessage: UserRemovalMessage) {}
function handleChangeRoomMessage(changeRoomMessage: ChangeRoomMessage) {}
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
            message_id: "", // Assign a unique ID if necessary
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
        const messageWrapper: HTMLElement = document.createElement('div');
        const usernameElement: HTMLElement = document.createElement('div');
        const messageElement: HTMLElement = document.createElement('div');

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


const textarea = document.querySelector('textarea[name="message_form"]') as HTMLTextAreaElement;
if (textarea) {
    textarea.addEventListener('keydown', function(event: KeyboardEvent) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
}

document.getElementById('Username')!.addEventListener('submit', function(event: Event) {
    event.preventDefault();
    const usernameField: HTMLInputElement = document.getElementById('usernameField')! as HTMLInputElement;
    const new_username: string = usernameField.value.trim();
    if (new_username !== '') {
        const usernameChangeMessage: UsernameChangeMessage = {
            new_username: new_username,
            sender_id: sender_id
        };
        const wrappedMessage: UserMessage = {
            UsernameChange: usernameChangeMessage
        };
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(wrappedMessage));
        }
    }
});

document.getElementById('Username')!.addEventListener('keydown', function(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        const usernameField: HTMLInputElement = document.getElementById('usernameField')! as HTMLInputElement;
        const new_username: string = usernameField.value.trim();
        if (new_username !== '') {
            const usernameChangeMessage: UsernameChangeMessage = {
                new_username: new_username,
                sender_id: sender_id
            };
            const wrappedMessage: UserMessage = {
                UsernameChange: usernameChangeMessage
            };
            if (socket.readyState === WebSocket.OPEN) {
                socket.send(JSON.stringify(wrappedMessage));
            }
        }
    }
});

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
                // Handle the response data here
            }).catch(error => {
                console.error('There has been a problem with your fetch operation:', error);
            });
        };
        reader.readAsDataURL(file);
    }
});
