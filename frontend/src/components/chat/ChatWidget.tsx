/* [064A-28+064A-52] ChatWidget: redesign completo. Sin header, burbuja con avatar,
 * fondo semi-transparente, sombra especifica del usuario, animacion suave.
 * [064A-52] Eliminada lógica de pedir nombre — el agente IA lo pide directamente.
 * Input 100% ancho con send button overlaid, bg unset, radius 104px.
 * Avatar de AI al lado de los mensajes en vez de icono Bot.
 * sentinel-disable-file limite-lineas: ChatWidget integra mensajes, input, attachments,
 * typing indicator y branding en un solo componente — lógica ya extraída a useChatWidget. */

import React, {useState, useRef, useEffect} from 'react';
import {useLocation} from 'react-router-dom';
import {Send, Minus, Paperclip, FileText, Palette, Package, AlertTriangle} from 'lucide-react';
import {useChatWidget} from '../../hooks/useChatWidget';
import {useChatStore} from '../../stores/chatStore';
import {Input} from '../ui/Input';
import {Button} from '../ui/Button';
import OptimizedImage from '../ui/OptimizedImage';
import './ChatWidget.css';

const AVATAR_SRC = '/assets/random/85a51ba9a4233272662e744b48f97d67.jpg';

export const ChatWidget: React.FC = () => {
    const abierto = useChatStore(s => s.abierto);
    const abrir = useChatStore(s => s.abrir);
    const cerrar = useChatStore(s => s.cerrar);
    const storeContext = useChatStore(s => s.context);
    const [input, setInput] = useState('');
    const messagesEndRef = useRef<HTMLDivElement>(null);

    const {
        connected,
        connecting,
        messages,
        typing,
        uploading,
        sessionId,
        connect,
        sendMessage,
        sendTyping,
        uploadFile,
    } = useChatWidget();

    /* [064A-67→074A-18][095A-21] Widget oculto en /panel sin cambiar el
     * orden de hooks. El return temprano antes de useEffect provocaba React #310
     * al entrar/salir del panel y dejaba el root vacío hasta recargar. */
    const location = useLocation();
    const enPanel = location.pathname.startsWith('/panel');

    useEffect(() => {
        /* [065A-1] El widget también se abre desde CTAs externos via store.
         * Si ya llega abierto al montar o cambia a abierto sin pasar por handleOpen,
         * igual debe iniciar el WebSocket del visitante.
         * [165A-20] En /panel no hay burbuja, pero un CTA explicito puede abrir el panel. */
        if (!abierto || connected || connecting) return;
        connect(undefined, storeContext);
    }, [abierto, connected, connecting, connect, storeContext]);

    if (enPanel && !abierto) return null;

    /* [064A-52] Al abrir, conectar directamente sin pedir nombre.
     * El agente IA pedirá el nombre al usuario si lo necesita.
     * [084A-28] Pasa el contexto del store al WS (hosting, servicio, etc.) */
    const handleOpen = () => {
        abrir();
        if (!connected && !connecting) {
            connect(undefined, storeContext);
        }
    };

    const handleMinimize = () => {
        cerrar();
    };

    const handleSend = () => {
        const content = input.trim();
        if (!content || !connected) return;
        sendMessage(content);
        setInput('');
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    };

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setInput(e.target.value);
        if (e.target.value.trim()) {
            sendTyping(e.target.value);
        }
    };

    return (
        <>
            {!enPanel && (
                <Button
                    variante="texto"
                    tamano="pequeno"
                    className={`chatWidgetBubble ${abierto ? 'chatWidgetBubbleOculta' : ''}`}
                    onClick={handleOpen}
                    aria-label="Abrir chat"
                    type="button"
                >
                    <OptimizedImage src={AVATAR_SRC} alt="" className="chatWidgetBubbleAvatar" loading="lazy" sizes="36px" />
                    <span className="chatWidgetBubbleTexto">Chat</span>
                </Button>
            )}

            <div className={`chatWidgetPanel ${abierto ? 'chatWidgetPanelAbierto' : ''}`}>
                <Button
                    variante="texto"
                    tamano="pequeno"
                    className="chatWidgetMinimizeBtn"
                    onClick={handleMinimize}
                    aria-label="Minimizar chat"
                    type="button"
                >
                    <Minus size={18} />
                </Button>

                {/* [114A-13] Banner de contexto de reporte: indica visualmente que el chat
                 * fue abierto desde "Reportar problema" en una orden específica. */}
                {storeContext?.startsWith('problem:') && (
                    <div className="chatWidgetReportBanner">
                        <AlertTriangle size={14} />
                        <span>Reportando un problema con tu pedido</span>
                    </div>
                )}

                <ChatWidgetMessages
                    messages={messages}
                    typing={typing}
                    messagesEndRef={messagesEndRef}
                />
                <ChatWidgetInput
                    input={input}
                    connected={connected && Boolean(sessionId)}
                    uploading={uploading}
                    onInputChange={handleInputChange}
                    onKeyDown={handleKeyDown}
                    onSend={handleSend}
                    onUpload={uploadFile}
                />
            </div>
        </>
    );
};

/* Sub-componentes (SRP) */

/* [T-5] Renderiza contenido rico de mensajes: imágenes inline, audio player, enlace PDF.
 * [084A-26] Agregados: invoice (botón pagar), service_card (tarjeta servicio), order_card.
 * Si message_type es null/text, renderiza texto plano como antes. */
function renderMessageContent(msg: {
    content: string;
    message_type?: string | null;
    metadata?: Record<string, unknown> | null;
}): React.ReactNode {
    const fileUrl = (msg.metadata?.file_url as string) || '';
    const fileName = (msg.metadata?.file_name as string) || 'archivo';

    switch (msg.message_type) {
        case 'image':
            return (
                <div className="chatWidgetMsgRich">
                    <a href={fileUrl} target="_blank" rel="noopener noreferrer">
                        <OptimizedImage
                            src={fileUrl}
                            alt={fileName}
                            className="chatWidgetMsgImage"
                            loading="lazy"
                        />
                    </a>
                    {msg.content && <p className="chatWidgetMsgCaption">{msg.content}</p>}
                </div>
            );
        case 'audio':
            return (
                <div className="chatWidgetMsgRich">
                    <audio controls preload="metadata" className="chatWidgetMsgAudio">
                        <source src={fileUrl} />
                    </audio>
                    {msg.content && <p className="chatWidgetMsgCaption">{msg.content}</p>}
                </div>
            );
        case 'file':
            return (
                <div className="chatWidgetMsgRich">
                    <a
                        href={fileUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="chatWidgetMsgFileLink"
                    >
                        <FileText size={14} /> {fileName}
                    </a>
                    {msg.content && <p className="chatWidgetMsgCaption">{msg.content}</p>}
                </div>
            );
        /* [084A-26+084A-38] Invoice: tarjeta con monto, descripción y botón de pago.
         * Status: "open" = pendiente de pago (mostrar botón), "paid" = pagada.
         * Stripe solo marca "paid" cuando el cliente paga — nunca al crear. */
        case 'invoice': {
            const payUrl = (msg.metadata?.payment_url as string) || '';
            const amountCents = (msg.metadata?.amount_cents as number) || 0;
            const currency = (msg.metadata?.currency as string) || 'usd';
            const description = (msg.metadata?.description as string) || '';
            const title = (msg.metadata?.title as string) || 'Factura';
            const status = (msg.metadata?.status as string) || '';
            const amountFormatted = (amountCents / 100).toFixed(2);
            const isPaid = status === 'paid';
            const isOpen = status === 'open' || status === '' || status === 'draft';

            return (
                <div className="chatWidgetMsgRich chatWidgetInvoiceCard">
                    <div className="chatWidgetInvoiceHeader">
                        <span className="chatWidgetInvoiceTitle">{title}</span>
                        {isPaid && <span className="chatWidgetInvoicePaid">Pagada</span>}
                        {isOpen && <span className="chatWidgetInvoicePending">Pendiente</span>}
                    </div>
                    <p className="chatWidgetInvoiceAmount">
                        ${amountFormatted} {currency.toUpperCase()}
                    </p>
                    {description && (
                        <p className="chatWidgetInvoiceDesc">{description}</p>
                    )}
                    {payUrl && !isPaid && (
                        <a
                            href={payUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="chatWidgetInvoicePayBtn"
                        >
                            Pagar ahora
                        </a>
                    )}
                    {msg.content && <p className="chatWidgetMsgCaption">{msg.content}</p>}
                </div>
            );
        }
        /* [084A-26] Service card: tarjeta de servicio con precio y botón */
        case 'service_card': {
            const title = (msg.metadata?.title as string) || '';
            const description = (msg.metadata?.description as string) || '';
            const basePriceCents = (msg.metadata?.base_price_cents as number) || 0;
            const priceFormatted = (basePriceCents / 100).toFixed(2);

            return (
                <div className="chatWidgetMsgRich chatWidgetServiceCard">
                    <div className="chatWidgetServiceHeader">
                        <span className="chatWidgetServiceIcon"><Palette size={16} /></span>
                        <span className="chatWidgetServiceTitle">{title}</span>
                    </div>
                    {description && (
                        <p className="chatWidgetServiceDesc">{description}</p>
                    )}
                    <p className="chatWidgetServicePrice">
                        Desde ${priceFormatted} USD
                    </p>
                    {msg.content && <p className="chatWidgetMsgCaption">{msg.content}</p>}
                </div>
            );
        }
        /* [084A-26] Order card: tarjeta de pedido con estado */
        case 'order_card': {
            const orderNumber = (msg.metadata?.order_number as string) || '';
            const serviceTitle = (msg.metadata?.service_title as string) || '';
            const orderStatus = (msg.metadata?.status as string) || '';

            return (
                <div className="chatWidgetMsgRich chatWidgetOrderCard">
                    <div className="chatWidgetOrderHeader">
                        <span className="chatWidgetOrderIcon"><Package size={16} /></span>
                        <span className="chatWidgetOrderTitle">Pedido #{orderNumber}</span>
                    </div>
                    {serviceTitle && (
                        <p className="chatWidgetOrderService">{serviceTitle}</p>
                    )}
                    {orderStatus && (
                        <p className="chatWidgetOrderStatus">Estado: {orderStatus}</p>
                    )}
                    {msg.content && <p className="chatWidgetMsgCaption">{msg.content}</p>}
                </div>
            );
        }
        /* [237A-7g] CTA de WhatsApp: botón para escribir al soporte cuando la IA escala */
        case 'contact_cta': {
            const label = (msg.metadata?.label as string) || 'Escribir por WhatsApp';
            const href = (msg.metadata?.href as string) || '';
            const fallback = (msg.metadata?.fallback as string) || 'El equipo fue notificado y responderá por este chat.';

            return (
                <div className="chatWidgetMsgRich chatWidgetContactCta">
                    <p className="chatWidgetContactCtaMsg">{msg.content}</p>
                    {href ? (
                        <a
                            href={href}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="chatWidgetContactCtaBtn"
                        >
                            {label}
                        </a>
                    ) : (
                        <p className="chatWidgetContactCtaFallback">{fallback}</p>
                    )}
                </div>
            );
        }
        default:
            return <>{msg.content}</>;
    }
}

function ChatWidgetMessages({
    messages,
    typing,
    messagesEndRef,
}: {
    messages: Array<{
        id: string;
        sender_type: string;
        sender_id: string | null;
        content: string;
        created_at: string;
        message_type?: string | null;
        metadata?: Record<string, unknown> | null;
    }>;
    typing: {sender: string; content: string} | null;
    messagesEndRef: React.RefObject<HTMLDivElement>;
}) {
    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({behavior: 'smooth'});
    }, [messages, typing, messagesEndRef]);

    return (
        <div className="chatWidgetMessages">
            {/* [124A-VISIT1] Saludo de bienvenida UI-only — siempre visible, sin BD ni sesión en panel.
             * El visitante ve el greeting al abrir. La sesión se crea solo al enviar el primer mensaje. */}
            <div className="chatWidgetMsg chatWidgetMsgOther">
                <div className="chatWidgetAiRow">
                    <OptimizedImage src={AVATAR_SRC} alt="" className="chatWidgetAiAvatar" loading="lazy" />
                    <div className="chatWidgetMsgBubble chatWidgetMsgBubbleOther">
                        ¡Hola! Estoy aquí para ayudarte. Puedes preguntarme acerca de nuestros servicios, resolver dudas, o cualquier consulta que tengas.
                    </div>
                </div>
            </div>
            {messages.map((msg) => {
                const isOwn = msg.sender_type === 'visitor' || msg.sender_type === 'client';

                /* [T-5] Renderizar contenido según message_type */
                const bubbleContent = renderMessageContent(msg);
                const bubble = (
                    <div className={`chatWidgetMsgBubble ${isOwn ? 'chatWidgetMsgBubbleOwn' : 'chatWidgetMsgBubbleOther'}`}>
                        {bubbleContent}
                    </div>
                );

                return (
                    <div
                        key={msg.id}
                        className={`chatWidgetMsg ${isOwn ? 'chatWidgetMsgOwn' : 'chatWidgetMsgOther'}`}
                    >
                        {/* [124A-WIDGET1] Todos los mensajes "other" (IA y staff) usan el mismo
                         * estilo con avatar — el cliente no debe distinguir entre IA y humano. */}
                        {!isOwn ? (
                            <div className="chatWidgetAiRow">
                                <OptimizedImage src={AVATAR_SRC} alt="" className="chatWidgetAiAvatar" loading="lazy" />
                                {bubble}
                            </div>
                        ) : (
                            bubble
                        )}
                    </div>
                );
            })}

            {typing && (
                <div className="chatWidgetMsg chatWidgetMsgOther">
                    {/* [124A-WIDGET1] Typing indicator también usa chatWidgetAiRow para todos */}
                    <div className="chatWidgetAiRow">
                        <OptimizedImage src={AVATAR_SRC} alt="" className="chatWidgetAiAvatar" loading="lazy" />
                        <div className="chatWidgetMsgBubble chatWidgetMsgBubbleOther chatWidgetTyping">
                            <span className="chatWidgetTypingDots">
                                <span />
                                <span />
                                <span />
                            </span>
                        </div>
                    </div>
                </div>
            )}

            <div ref={messagesEndRef} />
        </div>
    );
}

/* [T-5] MIME types aceptados en el file picker */
const CHAT_FILE_ACCEPT = 'image/jpeg,image/png,image/webp,image/gif,audio/mpeg,audio/ogg,audio/wav,audio/webm,audio/mp4,audio/flac,application/pdf';

function ChatWidgetInput({
    input,
    connected,
    uploading,
    onInputChange,
    onKeyDown,
    onSend,
    onUpload,
}: {
    input: string;
    connected: boolean;
    uploading: boolean;
    onInputChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
    onKeyDown: (e: React.KeyboardEvent) => void;
    onSend: () => void;
    onUpload: (file: File) => void;
}) {
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
            onUpload(file);
            e.target.value = '';
        }
    };

    return (
        <div className="chatWidgetInputArea">
            <input
                ref={fileInputRef}
                type="file"
                accept={CHAT_FILE_ACCEPT}
                onChange={handleFileChange}
                className="chatWidgetFileInput"
            />
            <Button
                variante="texto"
                tamano="pequeno"
                className="chatWidgetAttachBtn"
                onClick={() => fileInputRef.current?.click()}
                disabled={!connected || uploading}
                aria-label="Adjuntar archivo"
                type="button"
            >
                <Paperclip size={18} />
            </Button>
            <div className="chatWidgetInputWrapper">
                <Input
                    type="text"
                    placeholder={uploading ? 'Subiendo archivo...' : connected ? 'Escribe un mensaje...' : 'Conectando...'}
                    value={input}
                    onChange={onInputChange}
                    onKeyDown={onKeyDown}
                    disabled={!connected || uploading}
                    className="chatWidgetInput"
                />
                <Button
                    variante="texto"
                    tamano="pequeno"
                    className="chatWidgetSendBtn"
                    onClick={onSend}
                    disabled={!connected || !input.trim() || uploading}
                    aria-label="Enviar mensaje"
                    type="button"
                >
                    <Send size={18} />
                </Button>
            </div>
        </div>
    );
}
