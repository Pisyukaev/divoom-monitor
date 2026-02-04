using System.Text.Json;
using LibreHardwareMonitor.Hardware;

var computer = new Computer
{
    IsCpuEnabled = true,
    IsGpuEnabled = true
};

computer.Open();

float? cpuTemp = null;
float? gpuTemp = null;

foreach (var hardware in computer.Hardware)
{
    hardware.Update();
    foreach (var sensor in hardware.Sensors)
    {
        if (sensor.SensorType != SensorType.Temperature || sensor.Value is null)
        {
            continue;
        }

        var value = sensor.Value.Value;
        switch (hardware.HardwareType)
        {
            case HardwareType.Cpu:
                cpuTemp = cpuTemp.HasValue ? Math.Max(cpuTemp.Value, value) : value;
                break;
            case HardwareType.GpuAmd:
            case HardwareType.GpuNvidia:
            case HardwareType.GpuIntel:
                gpuTemp = gpuTemp.HasValue ? Math.Max(gpuTemp.Value, value) : value;
                break;
        }
    }
}

var payload = new
{
    cpu_temperature = cpuTemp,
    gpu_temperature = gpuTemp
};

Console.WriteLine(JsonSerializer.Serialize(payload));
