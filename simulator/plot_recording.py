from pathlib import Path

import matplotlib.pyplot as plt
import pandas as pd


PROJECT_ROOT = Path(__file__).resolve().parents[1]
INPUT_CSV = PROJECT_ROOT / "recordings" / "session.csv"
OUTPUT_DIR = PROJECT_ROOT / "figures"

OUTPUT_DIR.mkdir(exist_ok=True)

data = pd.read_csv(INPUT_CSV)

time_seconds = (
    data["timestamp_microseconds"]
    - data["timestamp_microseconds"].iloc[0]
) / 1_000_000

channel_columns = [
    f"channel_{channel_number}"
    for channel_number in range(1, 9)
]

plt.figure(figsize=(12, 6))

for channel_column in channel_columns:
    plt.plot(
        time_seconds,
        data[channel_column],
        label=channel_column.replace("_", " ").title(),
        linewidth=1,
    )

plt.xlabel("Time (seconds)")
plt.ylabel("Spike count per packet")
plt.title("Simulated Neural Recording")
plt.legend(ncol=4, fontsize=8)
plt.tight_layout()

spike_figure_path = OUTPUT_DIR / "neural_channels.png"
plt.savefig(spike_figure_path, dpi=200)
plt.close()

plt.figure(figsize=(12, 4))
plt.plot(time_seconds, data["temperature_celsius"])
plt.xlabel("Time (seconds)")
plt.ylabel("Temperature (°C)")
plt.title("Device Temperature Telemetry")
plt.tight_layout()

temperature_figure_path = OUTPUT_DIR / "temperature.png"
plt.savefig(temperature_figure_path, dpi=200)
plt.close()

plt.figure(figsize=(12, 4))
plt.plot(time_seconds, data["battery_percent"])
plt.xlabel("Time (seconds)")
plt.ylabel("Battery (%)")
plt.title("Device Battery Telemetry")
plt.ylim(0, 105)
plt.tight_layout()

battery_figure_path = OUTPUT_DIR / "battery.png"
plt.savefig(battery_figure_path, dpi=200)
plt.close()

print(f"Loaded {len(data)} packets from {INPUT_CSV}")
print(f"Saved: {spike_figure_path}")
print(f"Saved: {temperature_figure_path}")
print(f"Saved: {battery_figure_path}")