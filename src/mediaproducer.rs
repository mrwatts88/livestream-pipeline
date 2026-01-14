use gst::glib::MainLoop;
use gst::prelude::*;
use gst::{Element, ElementFactory, MessageView, Pipeline, State};
use gstreamer::glib::{ControlFlow, source};
use gstreamer::{self as gst, Promise};
use gstreamer_webrtc::WebRTCSessionDescription;
use gstreamer_webrtc::gst_sdp::SDPMessage;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::Signal;

pub fn run_producer_pipeline(send_to_tokio: Sender<Signal>, mut gst_recv: Receiver<Signal>) {
    gst::init().unwrap();

    let pipeline = Pipeline::with_name("pipeline");

    // todo: swap uridecodebin for camera
    let decode_bin = ElementFactory::make("uridecodebin")
        .property(
            "uri",
            "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
        )
        .build()
        .unwrap();

    let audio_converter = ElementFactory::make("audioconvert").build().unwrap();
    let video_converter = ElementFactory::make("videoconvert").build().unwrap();
    let audio_resampler = ElementFactory::make("audioresample").build().unwrap();
    let audio_encoder = ElementFactory::make("opusenc").build().unwrap();
    let video_encoder = ElementFactory::make("x264enc").build().unwrap();
    let audio_payloader = ElementFactory::make("rtpopuspay").build().unwrap();
    let video_payloader = ElementFactory::make("rtph264pay").build().unwrap();
    let webrtc_bin = ElementFactory::make("webrtcbin")
        .property_from_str("stun-server", "stun://stun.l.google.com:19302")
        .build()
        .unwrap();

    pipeline
        .add_many([
            &decode_bin,
            &audio_converter,
            &video_converter,
            &audio_resampler,
            &audio_encoder,
            &video_encoder,
            &audio_payloader,
            &video_payloader,
            &webrtc_bin,
        ])
        .unwrap();

    Element::link_many([
        &audio_converter,
        &audio_resampler,
        &audio_encoder,
        &audio_payloader,
        &webrtc_bin,
    ])
    .unwrap();
    Element::link_many([
        &video_converter,
        &video_encoder,
        &video_payloader,
        &webrtc_bin,
    ])
    .unwrap();

    webrtc_bin.connect_notify(None, |x, y| {
        println!("notify called");
        println!("{:?}", y.name());
        let sig = x.property::<gstreamer_webrtc::WebRTCSignalingState>("signaling-state");
        let ice = x.property::<gstreamer_webrtc::WebRTCICEGatheringState>("ice-gathering-state");
        println!("signaling-state: {:?}", sig);
        println!("ice-gathering-state: {:?}", ice);
    });

    let webrtc_bin_clone = webrtc_bin.clone();
    let sender_clone = send_to_tokio.clone();
    webrtc_bin.connect("on-negotiation-needed", false, move |_| {
        println!("Negotiation needed from webrtcbin");
        let webrtc_bin_clone = webrtc_bin_clone.clone();
        let webrtc_bin_clone_clone = webrtc_bin_clone.clone();
        let sender_clone = sender_clone.clone();
        let promise = Promise::with_change_func(move |res| {
            let option = res.unwrap();

            if let Some(val) = option {
                let offer = val
                    .get::<gstreamer_webrtc::WebRTCSessionDescription>("offer")
                    .unwrap();

                println!(
                    "Got offer from webrtcbin, setting local description and sending Signal::Offer to tokio."
                );
                webrtc_bin_clone
                    .emit_by_name::<()>("set-local-description", &[&offer, &None::<gst::Promise>]);

                sender_clone
                    .blocking_send(Signal::Offer(offer.sdp().as_text().unwrap()))
                    .unwrap();

                println!("Sent to tokio.");
            }
        });

        // tell webrtcbin to create an offer via an event
        println!("Telling webrtcbin to create an offer");
        webrtc_bin_clone_clone
            .emit_by_name::<()>("create-offer", &[&None::<gst::Structure>, &promise.clone()]);
        None
    });

    let sender_clone = send_to_tokio.clone();
    webrtc_bin.connect("on-ice-candidate", false, move |values| {
        println!("on ice candidate event, sending to peer.");

        let _webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
        let mline_index = values[1].get::<u32>().expect("Invalid argument");
        let candidate = values[2].get::<String>().expect("Invalid argument");

        println!("mline_index: {}", mline_index);
        println!("candidate: {}", candidate);

        sender_clone
            .blocking_send(Signal::IceCandidate {
                mline_index,
                candidate,
            })
            .unwrap();

        None
    });

    decode_bin.connect_pad_added(move |_src, src_pad| {
        let new_pad_caps = src_pad.current_caps().unwrap();
        let new_pad_struct = new_pad_caps.structure(0).unwrap();
        let new_pad_type = new_pad_struct.name();

        let is_video = new_pad_type.starts_with("video/x-raw");
        let is_audio = new_pad_type.starts_with("audio/x-raw");

        let el: Option<&Element> = if is_audio {
            Some(&audio_converter)
        } else if is_video {
            Some(&video_converter)
        } else {
            None
        };

        if let Some(el) = el {
            let converter_sink_pad = el.static_pad("sink").unwrap();

            if !converter_sink_pad.is_linked() {
                let res = src_pad.link(&converter_sink_pad);

                if res.is_err() {
                    eprintln!("Error linking pad");
                } else {
                    println!("Converter sink pad linked");
                }
            }
        }
    });

    let webrtc_bin_clone = webrtc_bin.clone();
    source::idle_add(move || {
        let msg_result = gst_recv.try_recv();

        if let Ok(signal) = msg_result {
            match signal {
                Signal::IceCandidate {
                    mline_index,
                    candidate,
                } => {
                    println!(
                        "Ice candidate received. mline_index: {}, candidate: {}. setting on webrtcbin.",
                        mline_index, candidate
                    );

                    webrtc_bin_clone
                        .emit_by_name::<()>("add-ice-candidate", &[&mline_index, &candidate]);
                }
                Signal::Answer(sdp) => {
                    println!("got answer from consumer. setting remote description.");
                    let answer = WebRTCSessionDescription::new(
                        gstreamer_webrtc::WebRTCSDPType::Answer,
                        SDPMessage::parse_buffer(sdp.as_bytes()).unwrap(),
                    );

                    webrtc_bin_clone.emit_by_name::<()>(
                        "set-remote-description",
                        &[&answer, &None::<gst::Promise>],
                    );
                }
                Signal::Offer(_sdp) => {
                    println!("should not get offer in producer.");
                }
            }
        }

        ControlFlow::Continue
    });

    let main_loop = MainLoop::new(None, false);
    let main_loop_clone = main_loop.clone();
    let bus = pipeline.bus().unwrap();

    bus.connect_message(Some("error"), move |_, msg| match msg.view() {
        MessageView::Error(err) => {
            eprintln!("Error message received from bus: {:?}", err);
            main_loop_clone.quit();
        }
        MessageView::Eos(..) => {
            main_loop_clone.quit();
        }
        _ => unreachable!(),
    });

    bus.add_signal_watch();
    pipeline.set_state(State::Playing).unwrap();

    main_loop.run();
    pipeline.set_state(State::Null).unwrap();
    bus.remove_signal_watch();
}
