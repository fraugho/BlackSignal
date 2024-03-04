let sender_id = "";
let user_map: { [key: string]: string } = {};
let ws_id = "";
let username = "";
let current_room = "";
let room_map: { [key: string]: string} = {};

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

interface TSBasicMessage {
    content: string;
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
    | { TSBasic: TSBasicMessage }
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
                handle_initialization(data.Initialization);
                break;
            case 'Basic' in data:
                handle_basic_message(data.Basic);
                break;
            case 'Image' in data:
                handle_image_message(data.Image);
                break;
            case 'Notification' in data:
                handle_notification_message(data.Notification);
                break;
            case 'Typing' in data:
                handle_typing_message(data.Typing);
                break;
            case 'Deletion' in data:
                handle_message_deletion_message(data.Deletion);
                break;
            case 'NewUser' in data:
                handle_new_user_message(data.NewUser);
                break;
            case 'UserAddition' in data:
                handle_user_addition_message(data.UserAddition);
                break;
            case 'UserRemoval' in data:
                handle_user_removal_message(data.UserRemoval);
                break;
            case 'ChangeRoom' in data:
                handle_room_change_message(data.ChangeRoom);
                break;
            case 'UsernameChange' in data:
                handle_username_change_message(data.UsernameChange);
                break;
            case 'CreateRoomChange' in data:
                handle_create_room_message(data.CreateRoomChange);
                break;
            default:
                console.error("Unknown message type received");
        }
    };
}

function handle_initialization(init_message: InitMessage) {
    sender_id = init_message.user_id;
    ws_id = init_message.ws_id;
    username = init_message.username;
    user_map = init_message.user_map;
}

function handle_basic_message(message: BasicMessage) {
    if (message.ws_id === ws_id) {
        const chatContainer = document.getElementById('chat-container')!;
        const sentContainers = chatContainer.getElementsByClassName('sent-container');
        if (sentContainers.length > 0) {
            const lastSentContainer = sentContainers[sentContainers.length - 1];
            lastSentContainer.setAttribute('data-message-id', message.message_id);
        }
        return;
    }
    
    const chatContainer = document.getElementById('chat-container')!;
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
    } else {
        messageWrapper.classList.add('received-message');
        messageContainer.classList.add('received-container');
    }

    messageContainer.appendChild(messageWrapper);
    chatContainer.appendChild(messageContainer);
    chatContainer.scrollTop = chatContainer.scrollHeight;
}



let is_delete_mode = false; // Keeps track of whether delete mode is active

// Add this after the WebSocket initialization
document.getElementById('toggle-delete-mode')?.addEventListener('click', () => {
    is_delete_mode = !is_delete_mode;
    // Update the checkbox visibility
    const checkboxes = document.querySelectorAll('.delete-checkbox') as NodeListOf<HTMLInputElement>;
    checkboxes.forEach(checkbox => {
        checkbox.style.display = is_delete_mode ? 'block' : 'none';
    });

    // Optionally change the button text to indicate the mode
    const toggleDeleteButton = document.getElementById('toggle-delete-mode') as HTMLButtonElement;
    toggleDeleteButton.textContent = is_delete_mode ? 'Exit Delete Mode' : 'Enter Delete Mode';

    // Show/hide the delete selected messages button
    const deleteSelectedButton = document.getElementById('delete-selected-messages') as HTMLButtonElement;
    deleteSelectedButton.style.display = is_delete_mode ? 'block' : 'none';
});

document.getElementById('delete-selected-messages')?.addEventListener('click', () => {
    const selectedCheckboxes = document.querySelectorAll('.delete-checkbox:checked') as NodeListOf<HTMLInputElement>;

    selectedCheckboxes.forEach(checkbox => {
        const messageContainer = checkbox.closest('.message-container');
        if (messageContainer) {
            const messageId = messageContainer.getAttribute('data-message-id');
            if (messageId) {
                delete_message(messageId);
                messageContainer.remove();
            }
        }
    });

    if (is_delete_mode) {
        document.getElementById('toggle-delete-mode')?.click();
    }
});


function handle_image_message(message: ImageMessage) {}
function handle_notification_message(message: NotificationMessage) {}
function handle_typing_message(typingMessage: TypingMessage) {}
function handle_new_user_message(message: NewUserMessage) {
    user_map[message.user_id] = message.username;
}
function handle_user_addition_message(message: UserAdditionMessage) {}
function handle_user_removal_message(message: UserRemovalMessage) {}
function handle_room_change_message(message: ChangeRoomMessage) {}
function handle_message_deletion_message(message: DeletionMessage) {
    // Extract the message_id from the deletion message
    const messageId = message.message_id;

    // Find the message element in the DOM
    const messageElement = document.querySelector(`[data-message-id="${messageId}"]`);

    // If the element exists, remove it
    if (messageElement) {
        messageElement.remove();
    }
}
function handle_username_change_message(username_change_message: UsernameChangeMessage) {
    retroactively_change_username(user_map[username_change_message.sender_id],username_change_message.new_username);
    user_map[username_change_message.sender_id] = username_change_message.new_username;
    if (sender_id === username_change_message.sender_id){
        username = username_change_message.new_username;
    }
}

function retroactively_change_username(old_username: string, new_username: string): void {
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

function handle_create_room_message(createRoomChangeMessage: CreateRoomChangeMessage) {}

document.getElementById('Message')!.addEventListener('submit', function(event: Event) {
    event.preventDefault();
    send_message();
});

function send_message(): void {
    const textarea: HTMLTextAreaElement = document.querySelector('textarea[name="message_form"]')!;
    const message_content: string = textarea.value.trim();
    if (message_content !== '') {
        const basic_message: TSBasicMessage = {
            content: message_content,
        };
    
        const wrappedMessage: UserMessage = {
            TSBasic: basic_message
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
        messageElement.textContent = message_content;
        
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
            send_message();
        }
    });
}

function send_username_change(new_username: string): void {
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
        display_message('Username change successful!', 'success'); // Show success message
    })
    .catch(error => {
        console.error('There has been a problem with your fetch operation:', error.message);
        display_message(error.message, 'error'); // Show error message
    });    
}

function display_message(message: string, type: 'success' | 'error'): void {
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
    const username_field: HTMLInputElement = document.getElementById('username_field')! as HTMLInputElement;
    const new_username: string = username_field.value.trim();
    if (new_username !== '') {
        send_username_change(new_username);
    }
});

document.getElementById('Username')!.addEventListener('keydown', function(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        const username_field: HTMLInputElement = document.getElementById('username_field')! as HTMLInputElement;
        const new_username: string = username_field.value.trim();
        if (new_username !== '') {
            send_username_change(new_username);
        }
    }
});

function delete_message(message_id: string): void {

    const message: DeletionMessage = {
        sender_id: sender_id,
        message_id: message_id
    };

    const wrapped_message: UserMessage = {
        Deletion: message
    };


    if (socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(wrapped_message));
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