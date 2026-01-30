import { useEffect, useState } from "react";
import { listen, emit } from "@tauri-apps/api/event";

type ButtonEvent = {
  index: number;
  pressed: boolean;
};

type EncoderEvent = {
  index: number;
  delta: number;
};

type TouchSwipeEvent = {
  start: [number, number];
  end: [number, number];
};

export default function App() {
  const [events, setEvents] = useState<string[]>([]);

  useEffect(() => {
    console.log("Frontend mounting, registering listeners");

    const unlistenButton = listen<ButtonEvent>("streamdeck:button", (e) => {
      setEvents((prev) => [
        `Button ${e.payload.index} ${e.payload.pressed ? "DOWN" : "UP"}`,
        ...prev,
      ]);
    });

    const unlistenEncoder = listen<EncoderEvent>("streamdeck:encoder", (e) => {
      setEvents((prev) => [
        `Encoder ${e.payload.index} delta ${e.payload.delta}`,
        ...prev,
      ]);
    });

    const unlistenSwipe = listen<TouchSwipeEvent>("streamdeck:swipe", (e) => {
      const [sx, sy] = e.payload.start;
      const [ex, ey] = e.payload.end;
      setEvents((prev) => [`Swipe (${sx}, ${sy}) → (${ex}, ${ey})`, ...prev]);
    });

    const unlistenEncoderPress = listen<ButtonEvent>(
      "streamdeck:encoder-press",
      (e) => {
        setEvents((prev) => [
          `Encoder ${e.payload.index} ${
            e.payload.pressed ? "PRESS" : "RELEASE"
          }`,
          ...prev,
        ]);
      },
    );

    // ✅ NOW signal readiness
    emit("frontend-ready");
    console.log("Frontend ready signal sent");

    return () => {
      unlistenButton.then((f) => f());
      unlistenEncoder.then((f) => f());
      unlistenSwipe.then((f) => f());
      unlistenEncoderPress.then((f) => f());
    };
  }, []);

  return (
    <div style={{ padding: 20, fontFamily: "monospace" }}>
      <h1>ArchDeck</h1>
      <ul>
        {events.slice(0, 20).map((e, i) => (
          <li key={i}>{e}</li>
        ))}
      </ul>
    </div>
  );
}
