import React from "react";

type Item = {
  src: string;
  label: string;
};

export function YouTubeList(props: { items: Item[] }) {
  const { items } = props;

  return (
    <div className="youtube-list">
      {items.map((item, index) => (
        <YouTubeEmbed key={index} item={item} />
      ))}
    </div>
  );
}

function YouTubeEmbed(props: { item: Item }) {
  const { item } = props;
  return (
    <div className="youtube-item">
      <iframe
        className="youtube-video"
        src={item.src}
        title="YouTube video player"
        frameBorder="0"
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        referrerPolicy="strict-origin-when-cross-origin"
        allowFullScreen
      ></iframe>
      <h2>{item.label}</h2>
    </div>
  );
}
