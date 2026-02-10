using System.Text.Json;
using LibreHardwareMonitor.Hardware;
using System.Net;
using System.Text;
using System.Collections.Generic;
using System.Linq;

var computer = new Computer
{
    IsCpuEnabled = true,
    IsGpuEnabled = true,
    IsMemoryEnabled = true,
    IsStorageEnabled = true,
    IsMotherboardEnabled = true
};

computer.Open();

// HTTP сервер на localhost:8765
var listener = new HttpListener();
listener.Prefixes.Add("http://localhost:8765/");

try
{
    listener.Start();
    Console.Error.WriteLine("LibreHardwareMonitor service started on http://localhost:8765");
}
catch (Exception ex)
{
    Console.Error.WriteLine($"Failed to start HTTP server: {ex.Message}");
    Environment.Exit(1);
    return;
}

while (true)
{
    try
    {
        var context = await listener.GetContextAsync();
        var request = context.Request;
        var response = context.Response;

        float? cpuTemp = null;
        float? gpuTemp = null;

        foreach (var hardware in computer.Hardware)
        {
            hardware.Update();
            
            foreach (var sensor in hardware.Sensors)
            {
                if (sensor.SensorType != SensorType.Temperature || !sensor.Value.HasValue)
                {
                    continue;
                }

                var value = sensor.Value.Value;
                var sensorName = sensor.Name?.ToLower() ?? "";
                
                // Фильтруем неправильные значения (диапазон: -30..200°C)
                if (value < -30 || value > 200)
                {
                    continue;
                }
                
                switch (hardware.HardwareType)
                {
                    case HardwareType.Cpu:
                        // Приоритет датчику "CPU Package" или содержащему "total"
                        if (sensorName.Contains("package") || sensorName.Contains("total"))
                        {
                            cpuTemp = value;
                        }
                        else if (!cpuTemp.HasValue)
                        {
                            cpuTemp = value;
                        }
                        break;
                        
                    case HardwareType.GpuAmd:
                    case HardwareType.GpuNvidia:
                    case HardwareType.GpuIntel:
                        // Приоритет датчику "GPU Core"
                        if (sensorName.Contains("core") && !sensorName.Contains("memory"))
                        {
                            gpuTemp = value;
                        }
                        else if (!gpuTemp.HasValue && !sensorName.Contains("memory") && !sensorName.Contains("junction"))
                        {
                            gpuTemp = value;
                        }
                        break;
                }
            }
        }
        
        // Поиск CPU температуры на материнской плате, если не найдена
        if (!cpuTemp.HasValue)
        {
            foreach (var hardware in computer.Hardware)
            {
                if (hardware.HardwareType == HardwareType.Motherboard)
                {
                    hardware.Update();
                    foreach (var sensor in hardware.Sensors)
                    {
                        if (sensor.SensorType == SensorType.Temperature && sensor.Value.HasValue)
                        {
                            var value = sensor.Value.Value;
                            var sensorName = sensor.Name?.ToLower() ?? "";
                            
                            if (value >= -30 && value <= 200)
                            {
                                if (sensorName.Contains("cpu") || sensorName.Contains("package") || 
                                    sensorName.Contains("tctl") || sensorName.Contains("tdie") || 
                                    sensorName.Contains("processor"))
                                {
                                    cpuTemp = value;
                                    break;
                                }
                            }
                        }
                    }
                }
                
                if (cpuTemp.HasValue)
                {
                    break;
                }
            }
        }

        var payload = new
        {
            cpu_temperature = cpuTemp,
            gpu_temperature = gpuTemp
        };

        var json = JsonSerializer.Serialize(payload);
        var buffer = Encoding.UTF8.GetBytes(json);

        response.ContentType = "application/json";
        response.ContentLength64 = buffer.Length;
        response.StatusCode = 200;

        await response.OutputStream.WriteAsync(buffer, 0, buffer.Length);
        response.OutputStream.Close();
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"Error handling request: {ex.Message}");
    }
}
