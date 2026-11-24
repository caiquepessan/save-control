/** O escudo com o check do Save Control. */
export function Logo({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 2.5 20 5.6v6.1c0 5-3.4 8.6-8 10.3-4.6-1.7-8-5.3-8-10.3V5.6z" fill="currentColor" />
      <path
        d="m8.4 12.2 2.4 2.4 4.8-4.8"
        fill="none"
        stroke="#fff"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
