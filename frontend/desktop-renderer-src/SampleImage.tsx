const pixels = Array.from({ length: 81 }, (_, position) => ({
  x: 95 + (position % 9) * 19,
  y: 26 + Math.floor(position / 9) * 19,
  color: ["#59b7b0", "#79c4e4", "#334a60", "#b4d7b5", "#d6b879"][
    (position * 7 + Math.floor(position / 9)) % 5
  ],
}));

export function SampleImage() {
  return (
    <svg
      className="sample-image"
      viewBox="0 0 360 220"
      role="img"
      aria-label="Illustrative pixel grid"
    >
      <title>Illustrative pixel grid</title>
      <rect width="360" height="220" rx="6" fill="#101c28" />
      {pixels.map((pixel) => (
        <rect
          key={`${pixel.x}:${pixel.y}`}
          x={pixel.x}
          y={pixel.y}
          width="15"
          height="15"
          rx="2"
          fill={pixel.color}
        />
      ))}
    </svg>
  );
}
