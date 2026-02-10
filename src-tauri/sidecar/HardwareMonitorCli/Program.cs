using System.Text.Json;
using System.Text.Json.Serialization;
using LibreHardwareMonitor.Hardware;
using System.Net;
using System.Text;
using System.Collections.Generic;
using System.Linq;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

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

        var metrics = new SystemMetrics();
        float? cpuTemp = null;
        float? gpuTemp = null;
        float cpuUsageTotal = 0;
        int cpuCoreCount = 0;

        // Получаем данные о памяти через Windows API
        var (memoryTotal, memoryUsed) = MemoryHelper.GetMemoryInfo();

        foreach (var hardware in computer.Hardware)
        {
            hardware.Update();
            
            foreach (var sensor in hardware.Sensors)
            {
                if (!sensor.Value.HasValue)
                {
                    continue;
                }

                var value = sensor.Value.Value;
                var sensorName = sensor.Name?.ToLower() ?? "";
                
                switch (sensor.SensorType)
                {
                    case SensorType.Temperature:
                        // Фильтруем неправильные значения (диапазон: -30..200°C)
                        if (value < -30 || value > 200)
                        {
                            continue;
                        }
                        
                        switch (hardware.HardwareType)
                        {
                            case HardwareType.Cpu:
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
                        break;
                        
                    case SensorType.Load:
                        if (hardware.HardwareType == HardwareType.Cpu)
                        {
                            if (sensorName.Contains("total"))
                            {
                                cpuUsageTotal = value;
                            }
                            else if (sensorName.StartsWith("cpu core #"))
                            {
                                cpuCoreCount++;
                            }
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
        
        // Получаем информацию о дисках
        var disks = new List<DiskUsage>();
        var drives = DriveInfo.GetDrives();
        foreach (var drive in drives)
        {
            try
            {
                if (drive.IsReady && drive.DriveType == DriveType.Fixed)
                {
                    var totalSpace = (ulong)drive.TotalSize;
                    var availableSpace = (ulong)drive.AvailableFreeSpace;
                    var usedSpace = totalSpace - availableSpace;
                    var usagePercent = totalSpace > 0 ? (float)usedSpace / totalSpace * 100 : 0;
                    
                    disks.Add(new DiskUsage
                    {
                        Name = drive.Name,
                        MountPoint = drive.RootDirectory.FullName,
                        TotalSpace = totalSpace,
                        AvailableSpace = availableSpace,
                        UsedSpace = usedSpace,
                        UsagePercent = usagePercent
                    });
                }
            }
            catch
            {
                // Пропускаем диски с ошибками
            }
        }

        metrics.CpuUsage = cpuUsageTotal;
        metrics.CpuTemperature = cpuTemp;
        metrics.GpuTemperature = gpuTemp;
        metrics.MemoryTotal = memoryTotal;
        metrics.MemoryUsed = memoryUsed;
        metrics.Disks = disks;

        var payload = metrics;

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

class DiskUsage
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";
    
    [JsonPropertyName("mount_point")]
    public string MountPoint { get; set; } = "";
    
    [JsonPropertyName("total_space")]
    public ulong TotalSpace { get; set; }
    
    [JsonPropertyName("available_space")]
    public ulong AvailableSpace { get; set; }
    
    [JsonPropertyName("used_space")]
    public ulong UsedSpace { get; set; }
    
    [JsonPropertyName("usage_percent")]
    public float UsagePercent { get; set; }
}

class SystemMetrics
{
    [JsonPropertyName("cpu_usage")]
    public float CpuUsage { get; set; }
    
    [JsonPropertyName("cpu_temperature")]
    public float? CpuTemperature { get; set; }
    
    [JsonPropertyName("gpu_temperature")]
    public float? GpuTemperature { get; set; }
    
    [JsonPropertyName("memory_total")]
    public ulong MemoryTotal { get; set; }
    
    [JsonPropertyName("memory_used")]
    public ulong MemoryUsed { get; set; }
    
    [JsonPropertyName("disks")]
    public List<DiskUsage> Disks { get; set; } = new();
}

[StructLayout(LayoutKind.Sequential)]
struct MEMORYSTATUSEX
{
    public uint dwLength;
    public uint dwMemoryLoad;
    public ulong ullTotalPhys;
    public ulong ullAvailPhys;
    public ulong ullTotalPageFile;
    public ulong ullAvailPageFile;
    public ulong ullTotalVirtual;
    public ulong ullAvailVirtual;
    public ulong ullAvailExtendedVirtual;
}

static class NativeMethods
{
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GlobalMemoryStatusEx(ref MEMORYSTATUSEX lpBuffer);
}

static class MemoryHelper
{
    public static (ulong total, ulong used) GetMemoryInfo()
    {
        var memStatus = new MEMORYSTATUSEX { dwLength = (uint)Marshal.SizeOf(typeof(MEMORYSTATUSEX)) };
        if (NativeMethods.GlobalMemoryStatusEx(ref memStatus))
        {
            return (memStatus.ullTotalPhys, memStatus.ullTotalPhys - memStatus.ullAvailPhys);
        }
        return (0, 0);
    }
}
