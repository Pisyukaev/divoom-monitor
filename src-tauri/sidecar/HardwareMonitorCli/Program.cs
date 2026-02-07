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

        // Обновляем данные о железе
        float? cpuTemp = null;
        float? gpuTemp = null;
        var cpuSensorCount = 0;
        var gpuSensorCount = 0;
        var cpuTemps = new List<float>();
        var gpuTemps = new List<float>();

        foreach (var hardware in computer.Hardware)
        {
            hardware.Update();
            Console.Error.WriteLine($"Hardware: {hardware.HardwareType} - {hardware.Name}");
            
            foreach (var sensor in hardware.Sensors)
            {
                if (sensor.SensorType != SensorType.Temperature || sensor.Value is null)
                {
                    continue;
                }

                var value = sensor.Value.Value;
                var sensorName = sensor.Name?.ToLower() ?? "";
                
                // Фильтруем явно неправильные значения (например, 255°C - это часто значение по умолчанию для неработающих датчиков)
                // Разумный диапазон для температур: от -30 до 150°C
                if (value < -30 || value > 150)
                {
                    Console.Error.WriteLine($"  Sensor: {sensor.Name} = {value}°C - FILTERED OUT (out of range -30..150°C)");
                    continue;
                }
                
                Console.Error.WriteLine($"  Sensor: {sensor.Name} = {value}°C (Type: {sensor.SensorType}, Hardware: {hardware.HardwareType})");
                
                switch (hardware.HardwareType)
                {
                    case HardwareType.Cpu:
                        cpuSensorCount++;
                        cpuTemps.Add(value);
                        // Приоритет датчику "CPU total" или "total", если он есть
                        if (sensorName.Contains("total"))
                        {
                            cpuTemp = value; // Используем значение напрямую, так как это основной датчик
                            Console.Error.WriteLine($"    -> CPU temperature updated (TOTAL sensor): {cpuTemp}°C");
                        }
                        else if (!cpuTemp.HasValue || cpuTemp.Value < value)
                        {
                            // Если нет "total", берем максимальное значение
                            cpuTemp = value;
                            Console.Error.WriteLine($"    -> CPU temperature updated: {cpuTemp}°C");
                        }
                        break;
                    case HardwareType.GpuAmd:
                    case HardwareType.GpuNvidia:
                    case HardwareType.GpuIntel:
                        gpuSensorCount++;
                        gpuTemps.Add(value);
                        // Для GPU приоритет датчику "GPU Core" или "Core"
                        if (sensorName.Contains("core") && !sensorName.Contains("memory"))
                        {
                            // GPU Core имеет приоритет - используем его значение напрямую
                            gpuTemp = value;
                            Console.Error.WriteLine($"    -> GPU temperature updated (CORE sensor): {gpuTemp}°C");
                        }
                        else if (sensorName.Contains("memory") || sensorName.Contains("junction"))
                        {
                            // Игнорируем Memory Junction - часто дает неверные значения
                            Console.Error.WriteLine($"    -> GPU sensor ignored (Memory Junction): {sensor.Name} = {value}°C");
                        }
                        else if (!gpuTemp.HasValue)
                        {
                            // Если нет Core датчика, используем первый валидный (например, Hot Spot)
                            gpuTemp = value;
                            Console.Error.WriteLine($"    -> GPU temperature updated: {gpuTemp}°C");
                        }
                        // Если уже есть значение от Core, игнорируем остальные
                        break;
                }
            }
        }
        
        // Для CPU пытаемся найти температуру через другие источники, если основной не найден
        if (!cpuTemp.HasValue)
        {
            Console.Error.WriteLine("CPU temperature not found in CPU hardware, searching in Motherboard...");
            // Ищем температуру в других типах оборудования или через другие датчики
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
                            
                            // Фильтруем неправильные значения
                            if (value < -30 || value > 150)
                            {
                                continue;
                            }
                            
                            // Ищем датчики, которые могут быть связаны с CPU
                            if ((sensorName.Contains("cpu") || sensorName.Contains("package") || 
                                 sensorName.Contains("core") || sensorName.Contains("tctl") ||
                                 sensorName.Contains("tdie") || sensorName.Contains("processor")) && 
                                value >= 0 && value <= 150)
                            {
                                Console.Error.WriteLine($"  Found CPU-related sensor on {hardware.HardwareType}: {sensor.Name} = {value}°C");
                                cpuTemps.Add(value);
                                cpuTemp = cpuTemp.HasValue ? Math.Max(cpuTemp.Value, value) : value;
                                cpuSensorCount++;
                            }
                        }
                    }
                }
            }
        }
        
        Console.Error.WriteLine($"Summary - CPU sensors found: {cpuSensorCount}, GPU sensors found: {gpuSensorCount}");
        if (cpuTemps.Count > 0)
        {
            Console.Error.WriteLine($"CPU temperatures collected: [{string.Join(", ", cpuTemps.Select(t => $"{t:F1}°C"))}]");
        }
        if (gpuTemps.Count > 0)
        {
            Console.Error.WriteLine($"GPU temperatures collected: [{string.Join(", ", gpuTemps.Select(t => $"{t:F1}°C"))}]");
        }
        Console.Error.WriteLine($"Final temperatures - CPU: {cpuTemp?.ToString("F1") ?? "null"}°C, GPU: {gpuTemp?.ToString("F1") ?? "null"}°C");

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
