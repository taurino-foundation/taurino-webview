use crate::events::{
    DownloadEvent, DragDropEvent, PageLoadEvent, SynthesizedEvent,
};
use crate::pending::PendingWebview;
use crate::types::{BackgroundThrottlingPolicy, ScrollBarStyle};
use crate::utils::{
    IpcHandler, NewWindowFeatures, NewWindowOpener, NewWindowResponse,
    WebContext, WebViewMetaData, WebviewBounds, WebviewIpcHandler,
    parse_proxy_url,
};

use crate::webview::{WebView, WebviewId};
use crate::{manager::Manager, wrapper::RectWrapper};
use dpi::{LogicalPosition, PhysicalPosition};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::{
    collections::HashSet,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tao::window::{Theme as TaoTheme, Window, WindowId};
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;
use wry::{DragDropEvent as WryDragDropEvent, WebViewBuilder};
use wry::{ScrollBarStyle as WryScrollBarStyle, WebContext as WryWebContext};

#[cfg(windows)]
use wry::WebViewExtWindows;

pub(crate) fn create_wry_webview(
    window_label: String,
    window: &Window,
    pending: PendingWebview,
    manager: &mut Manager,
) -> crate::Result<WebView> {
    let focused_webview = Arc::new(Mutex::new(None::<String>));
    let window_id = manager.window_id.clone();
    let webviews_store = manager.webviews_store();

    let id = manager.next_webview_id();
    let _event_id = manager.next_webview_event_id();

    /*     if !manager.webview_runtime_installed {
        #[cfg(all(not(debug_assertions), windows))]
        dialog::error(
            r#"Could not find the WebView2 Runtime.

        Make sure it is installed or download it from <A href="https://developer.microsoft.com/en-us/microsoft-edge/webview2">https://developer.microsoft.com/en-us/microsoft-edge/webview2</A>

        You may have it installed on another user account, but it is not available for this one.
        "#,
        );

        if cfg!(target_os = "macos") {
            log::warn!("WebKit webview runtime not found, attempting to create webview anyway.");
        } else {
            return Err(crate::Error::WebviewRuntimeNotInstalled);
        }
    } */
    #[allow(unused_mut)]
    let PendingWebview {
        label,
        kind,
        webview_attributes,
        url,
        ipc_handler,
        navigation_handler,
        new_window_handler,
        document_title_changed_handler,
        web_resource_request_handler: _,
        on_page_load_handler,
        download_handler,
        proxy_handler,
        uri_scheme_protocols,
    } = pending;

    let mut web_context = manager
        .web_context()
        .lock()
        .expect("poisoned WebContext store");
    let is_first_context = web_context.is_empty();
    let automation_enabled =
        std::env::var("TAURI_WEBVIEW_AUTOMATION").as_deref() == Ok("true");
    let web_context_key = webview_attributes.data_directory;

    let entry = web_context.entry(web_context_key.clone());
    let web_context = match entry {
        Occupied(occupied) => {
            let occupied = occupied.into_mut();
            occupied.referenced_by_webviews.insert(label.clone());
            occupied
        }
        Vacant(vacant) => {
            let mut web_context = WryWebContext::new(web_context_key.clone());
            web_context.set_allows_automation(if automation_enabled {
                is_first_context
            } else {
                false
            });
            vacant.insert(WebContext {
                inner: web_context,
                referenced_by_webviews: [label.clone()].into(),
                registered_custom_protocols: HashSet::new(),
            })
        }
    };

    let mut webview_builder =
        WebViewBuilder::new_with_web_context(&mut web_context.inner)
            .with_id(&label)
            .with_focused(webview_attributes.focus)
            .with_transparent(webview_attributes.transparent)
            .with_accept_first_mouse(webview_attributes.accept_first_mouse)
            .with_incognito(webview_attributes.incognito)
            .with_clipboard(webview_attributes.clipboard)
            .with_hotkeys_zoom(webview_attributes.zoom_hotkeys_enabled)
            .with_general_autofill_enabled(
                webview_attributes.general_autofill_enabled,
            );

    #[cfg(target_os = "macos")]
    if let Some(webview_configuration) =
        webview_attributes.webview_configuration
    {
        webview_builder =
            webview_builder.with_webview_configuration(webview_configuration);
    }

    if let Some(background_throttling) =
        webview_attributes.background_throttling
    {
        webview_builder = webview_builder.with_background_throttling(
            match background_throttling {
                BackgroundThrottlingPolicy::Disabled => {
                    wry::BackgroundThrottlingPolicy::Disabled
                }
                BackgroundThrottlingPolicy::Suspend => {
                    wry::BackgroundThrottlingPolicy::Suspend
                }
                BackgroundThrottlingPolicy::Throttle => {
                    wry::BackgroundThrottlingPolicy::Throttle
                }
            },
        );
    }

    if webview_attributes.javascript_disabled {
        webview_builder = webview_builder.with_javascript_disabled();
    }

    if let Some(color) = webview_attributes.background_color {
        webview_builder = webview_builder.with_background_color(color.into());
    }
    if let Some(navigation_handler) = navigation_handler {
        webview_builder = webview_builder.with_navigation_handler(move |url| {
            url.parse()
                .map(|url| navigation_handler(&url))
                .unwrap_or(true)
        });
    }

    if let Some(new_window_handler) = new_window_handler {
        let webviews_store = Arc::clone(&webviews_store);
        let current_window_id = window_id.clone();

        webview_builder = webview_builder.with_new_window_req_handler(
            move |url, features| {
                let Ok(url) = url.parse() else {
                    return wry::NewWindowResponse::Deny;
                };

                let response = new_window_handler(
                    url,
                    NewWindowFeatures::new(
                        features.size,
                        features.position,
                        NewWindowOpener {
                            webview: features.opener.webview,

                            #[cfg(windows)]
                            environment: features.opener.environment,

                            #[cfg(target_os = "macos")]
                            target_configuration: features
                                .opener
                                .target_configuration,
                        },
                    ),
                );

                match response {
                    NewWindowResponse::Allow => wry::NewWindowResponse::Allow,

                    NewWindowResponse::Deny => wry::NewWindowResponse::Deny,

                    #[cfg(not(any(target_os = "android", target_os = "ios")))]
                    NewWindowResponse::Create { window_id } => {
                        let Some(this_window_id) =
                            *current_window_id.lock().unwrap()
                        else {
                            return wry::NewWindowResponse::Deny;
                        };

                        if window_id != this_window_id {
                            return wry::NewWindowResponse::Deny;
                        }

                        let target_webview = {
                            let webviews = webviews_store
                                .lock()
                                .expect("poisoned webview manager");

                            webviews.values().next().cloned()
                        };

                        let Some(target_webview) = target_webview else {
                            return wry::NewWindowResponse::Deny;
                        };

                        wry::NewWindowResponse::Create {
                            #[cfg(target_os = "macos")]
                            webview: wry::WebViewExtMacOS::webview(
                                &*target_webview,
                            )
                            .as_super()
                            .into(),

                            #[cfg(any(
                                target_os = "linux",
                                target_os = "dragonfly",
                                target_os = "freebsd",
                                target_os = "netbsd",
                                target_os = "openbsd",
                            ))]
                            webview: target_webview.webview(),

                            #[cfg(windows)]
                            webview: target_webview.webview(),
                        }
                    }
                }
            },
        );
    }
    if let Some(document_title_changed_handler) = document_title_changed_handler
    {
        webview_builder = webview_builder
            .with_document_title_changed_handler(document_title_changed_handler)
    }

    let webview_bounds = if let Some(bounds) = webview_attributes.bounds {
        let bounds: RectWrapper = bounds.into();
        let bounds = bounds.0;

        let scale_factor = window.scale_factor();
        let position = bounds.position.to_logical::<f32>(scale_factor);
        let size = bounds.size.to_logical::<f32>(scale_factor);

        webview_builder = webview_builder.with_bounds(bounds);

        let window_size = window.inner_size().to_logical::<f32>(scale_factor);

        if webview_attributes.auto_resize {
            Some(WebviewBounds {
                x_rate: position.x / window_size.width,
                y_rate: position.y / window_size.height,
                width_rate: size.width / window_size.width,
                height_rate: size.height / window_size.height,
            })
        } else {
            None
        }
    } else {
        if kind {
            webview_builder = webview_builder.with_bounds(wry::Rect {
                position: LogicalPosition::new(0, 0).into(),
                size: window.inner_size().into(),
            });
            Some(WebviewBounds {
                x_rate: 0.,
                y_rate: 0.,
                width_rate: 1.,
                height_rate: 1.,
            })
        } else {
            None
        }
    };

    if let Some(download_handler) = download_handler {
        let download_handler_ = download_handler.clone();
        webview_builder =
            webview_builder.with_download_started_handler(move |url, path| {
                if let Ok(url) = url.parse() {
                    download_handler_(DownloadEvent::Requested {
                        url,
                        destination: path,
                    })
                } else {
                    false
                }
            });
        webview_builder = webview_builder.with_download_completed_handler(
            move |url, path, success| {
                if let Ok(url) = url.parse() {
                    download_handler(DownloadEvent::Finished {
                        url,
                        path,
                        success,
                    });
                }
            },
        );
    }

    if let Some(page_load_handler) = on_page_load_handler {
        webview_builder =
            webview_builder.with_on_page_load_handler(move |event, url| {
                let _ = url.parse().map(|url| {
                    page_load_handler(
                        url,
                        match event {
                            wry::PageLoadEvent::Started => {
                                PageLoadEvent::Started
                            }
                            wry::PageLoadEvent::Finished => {
                                PageLoadEvent::Finished
                            }
                        },
                    )
                });
            });
    }

    if let Some(user_agent) = webview_attributes.user_agent {
        webview_builder = webview_builder.with_user_agent(&user_agent);
    }

    if let Some(proxy_url) = webview_attributes.proxy_url {
        let config = parse_proxy_url(&proxy_url)?;

        webview_builder = webview_builder.with_proxy_config(config);
    }

    {
        if let Some(additional_browser_args) =
            webview_attributes.additional_browser_args
        {
            webview_builder = webview_builder
                .with_additional_browser_args(&additional_browser_args);
        }

        if let Some(environment) = webview_attributes.environment {
            webview_builder = webview_builder.with_environment(environment);
        }

        webview_builder = webview_builder.with_theme(match window.theme() {
            TaoTheme::Dark => wry::Theme::Dark,
            TaoTheme::Light => wry::Theme::Light,
            _ => wry::Theme::Light,
        });

        webview_builder = webview_builder.with_scroll_bar_style(
            match webview_attributes.scroll_bar_style {
                ScrollBarStyle::Default => WryScrollBarStyle::Default,
                ScrollBarStyle::FluentOverlay => {
                    WryScrollBarStyle::FluentOverlay
                }
                #[allow(unreachable_patterns)]
                _ => unreachable!(),
            },
        );
    }

    #[cfg(windows)]
    {
        webview_builder = webview_builder.with_browser_extensions_enabled(
            webview_attributes.browser_extensions_enabled,
        );
    }

    #[cfg(any(
        windows,
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        if let Some(path) = &webview_attributes.extensions_path {
            webview_builder = webview_builder.with_extensions_path(path);
        }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        if let Some(related_view) = webview_attributes.related_view {
            webview_builder = webview_builder.with_related_view(related_view);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if let Some(data_store_identifier) =
            &webview_attributes.data_store_identifier
        {
            webview_builder = webview_builder
                .with_data_store_identifier(*data_store_identifier);
        }
    }
    if let Some(handler) = proxy_handler.as_ref().cloned() {
        if webview_attributes.drag_drop_handler_enabled {
            let window_id_ = window_id.clone();

            webview_builder =
                webview_builder.with_drag_drop_handler(move |event| {
                    let event = match event {
                        WryDragDropEvent::Enter {
                            paths,
                            position: (x, y),
                        } => DragDropEvent::Enter {
                            paths,
                            position: PhysicalPosition::new(x as _, y as _),
                        },

                        WryDragDropEvent::Over { position: (x, y) } => {
                            DragDropEvent::Over {
                                position: PhysicalPosition::new(x as _, y as _),
                            }
                        }

                        WryDragDropEvent::Drop {
                            paths,
                            position: (x, y),
                        } => DragDropEvent::Drop {
                            paths,
                            position: PhysicalPosition::new(x as _, y as _),
                        },

                        WryDragDropEvent::Leave => DragDropEvent::Leave,

                        _ => return true,
                    };

                    let message = if !kind {
                        SynthesizedEvent::window_drag_drop(event)
                    } else {
                        SynthesizedEvent::webview_drag_drop(event)
                    };

                    let Some(window_id) = *window_id_.lock().unwrap() else {
                        return true;
                    };

                    handler(window_id, id, message);
                    true
                });
        }
    }

    #[cfg(target_os = "ios")]
    {
        if let Some(input_accessory_view_builder) =
            webview_attributes.input_accessory_view_builder
        {
            webview_builder = webview_builder
                .with_input_accessory_view_builder(move |webview| {
                    input_accessory_view_builder.0(webview)
                });
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(position) = &webview_attributes.traffic_light_position {
            webview_builder =
                webview_builder.with_traffic_light_inset(*position);
        }
    }

    webview_builder = webview_builder.with_ipc_handler(create_ipc_handler(
        kind,
        window_label,
        label.clone(),
        window_id.clone(),
        id,
        ipc_handler,
    ));

    for script in webview_attributes.initialization_scripts {
        webview_builder = webview_builder
            .with_initialization_script_for_main_only(
                script.script,
                script.for_main_frame_only,
            );
    }

    for (scheme, protocol) in uri_scheme_protocols {
        let scheme = scheme.clone();

        webview_builder = webview_builder.with_asynchronous_custom_protocol(
            scheme,
            move |webview_id, request, responder| {
                protocol(
                    webview_id,
                    request,
                    Box::new(move |response| responder.respond(response)),
                )
            },
        );
    }

    #[cfg(any(debug_assertions, feature = "devtools"))]
    {
        webview_builder = webview_builder
            .with_devtools(webview_attributes.devtools.unwrap_or(true));
    }

    if url != "about:blank" {
        webview_builder = webview_builder.with_url(&url);
    }

    let webview = match kind {
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        )))]
        true => {
            // only way to account for menu bar height, and also works for multiwebviews :)
            let vbox = window.default_vbox().unwrap();
            webview_builder.build_gtk(vbox)
        }
        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        ))]
        true => webview_builder.build_as_child(&window),
        false => {
            #[cfg(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            ))]
            let builder = webview_builder.build(&window);
            #[cfg(not(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "ios",
                target_os = "android"
            )))]
            let builder = {
                let vbox = window.default_vbox().unwrap();
                webview_builder.build_gtk(vbox)
            };
            builder
        }
    }
    .map_err(|e| crate::Error::CreateWebview(Box::new(e)))?;

    #[cfg(windows)]
    {
        let controller = webview.controller();

        let proxy_handler = proxy_handler.as_ref().cloned();
        let webview_id = id;

        if let Some(handler) = proxy_handler {
            use webview2_com::{
                ContainsFullScreenElementChangedEventHandler,
                FocusChangedEventHandler,
            };

            // GotFocus ---------------------------------------------------------
            {
                let handler = handler.clone();
                let label_ = label.clone();
                let focused_webview_ = focused_webview.clone();
                let window_id_ = window_id.clone();

                let mut got_focus_token = Default::default();

                unsafe {
                    controller.add_GotFocus(
                        &FocusChangedEventHandler::create(Box::new(
                            move |_, _| {
                                let mut focused_webview =
                                    focused_webview_.lock().unwrap();

                                // Multi-webview mode:
                                // If any webview is already focused, this is only a webview-to-webview focus change.
                                // If no webview is focused yet, this is a real window focus change.
                                let already_focused = focused_webview.is_some();

                                focused_webview.replace(label_.clone());

                                if !already_focused {
                                    let Some(window_id) =
                                        *window_id_.lock().unwrap()
                                    else {
                                        return Ok(());
                                    };

                                    handler(
                                        window_id,
                                        webview_id,
                                        SynthesizedEvent::window_focus_changed(
                                            true,
                                        ),
                                    );
                                }

                                Ok(())
                            },
                        )),
                        &mut got_focus_token,
                    )
                }
                .unwrap();
            }

            // LostFocus --------------------------------------------------------
            {
                let handler = handler.clone();
                let label_ = label.clone();
                let focused_webview_ = focused_webview.clone();
                let window_id_ = window_id.clone();

                let mut lost_focus_token = Default::default();

                unsafe {
                    controller.add_LostFocus(
                        &FocusChangedEventHandler::create(Box::new(
                            move |_, _| {
                                let mut focused_webview =
                                    focused_webview_.lock().unwrap();

                                // Multi-webview mode:
                                // If another webview got focus before this LostFocus event,
                                // focused_webview contains the other webview label.
                                // Then this is NOT a real window focus loss.
                                let lost_window_focus = focused_webview
                                    .as_ref()
                                    .map_or(true, |w| w == &label_);

                                if lost_window_focus {
                                    // Only reset when the whole window lost focus.
                                    // Otherwise another webview is focused now.
                                    *focused_webview = None;

                                    let Some(window_id) =
                                        *window_id_.lock().unwrap()
                                    else {
                                        return Ok(());
                                    };

                                    handler(
                                        window_id,
                                        webview_id,
                                        SynthesizedEvent::window_focus_changed(
                                            true,
                                        ),
                                    );
                                }

                                Ok(())
                            },
                        )),
                        &mut lost_focus_token,
                    )
                }
                .unwrap();
            }

            // Fullscreen -------------------------------------------------------
            if let Ok(core_webview) = unsafe { controller.CoreWebView2() } {
                let handler = handler.clone();
                let window_id_ = window_id.clone();

                let mut fullscreen_token = Default::default();

                unsafe {
                    let _ = core_webview.add_ContainsFullScreenElementChanged(
                        &ContainsFullScreenElementChangedEventHandler::create(
                            Box::new(move |sender, _| {
                                let mut contains_fullscreen_element =
                                    windows::core::BOOL::default();

                                sender
                                    .ok_or_else(windows::core::Error::empty)?
                                    .ContainsFullScreenElement(
                                        &mut contains_fullscreen_element,
                                    )?;

                                let Some(window_id) =
                                    *window_id_.lock().unwrap()
                                else {
                                    return Ok(());
                                };

                                handler(
                                    window_id,
                                    webview_id,
                                    SynthesizedEvent::window_fullscreen_changed(
                                        contains_fullscreen_element.as_bool(),
                                    ),
                                );
                                Ok(())
                            }),
                        ),
                        &mut fullscreen_token,
                    );
                }
            }
        }
    }

    Ok(WebView {
        label,
        id,
        inner: Rc::new(webview),
        context_store: manager.web_context().clone(),
        /*  webview_event_listeners: Default::default(), */
        context_key: if automation_enabled {
            None
        } else {
            web_context_key
        },
        bounds: Arc::new(Mutex::new(webview_bounds)),
    })
}

/// Create a wry ipc handler from a tauri ipc handler.
fn create_ipc_handler(
    kind: bool,
    window_label: String,
    webview_label: String,
    window_id: Arc<Mutex<Option<WindowId>>>,
    webview_id: WebviewId,
    ipc_handler: Option<WebviewIpcHandler>,
) -> Box<IpcHandler> {
    Box::new(move |request| {
        if let Some(handler) = &ipc_handler {
            let window_id_guard = window_id.lock().unwrap();

            let Some(window_id) = *window_id_guard else {
                return;
            };

            let metadata = WebViewMetaData::new(
                &kind,
                window_label.as_str(),
                webview_label.as_str(),
                &window_id,
                &webview_id,
            );

            handler(metadata, request);
        }
    })
}
