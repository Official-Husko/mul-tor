sites_data_dict = {
    "PixelDrain": {
        "apiKey": False,
        "url": "https://pixeldrain.com/api/file/",
        "api_url": "https://pixeldrain.com/api/",
        "download_url_base": "https://pixeldrain.com/u/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "GoFile": {
        "apiKey": False,
        "url": "https://{server}.gofile.io/uploadFile",
        "api_url": "https://gofile.io/api/",
        "download_url_base": "https://gofile.io/d/",
        "server_url": "https://api.gofile.io/getServer"
    },
    "AnonFiles": {
        "apiKey": False,
        "url": "https://api.anonfiles.com/upload",
        "api_url": "https://api.anonfiles.com",
        "download_url_base": "https://anonfiles.com/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    }
}

class Site_Data_CLSS:
    
    def size_unit_calc(site_name, file_size):
        site_name = site_name
        size_limit = sites_data_dict[site_name]["size_limit_bytes"]
        
        step_to_greater_unit = 1024.
        number_of_bytes = float(file_size)
        unit = 'bytes'
        if (number_of_bytes / step_to_greater_unit) >= 1:
            number_of_bytes /= step_to_greater_unit
            unit = 'KB'
        if (number_of_bytes / step_to_greater_unit) >= 1:
            number_of_bytes /= step_to_greater_unit
            unit = 'MB'
        if (number_of_bytes / step_to_greater_unit) >= 1:
            number_of_bytes /= step_to_greater_unit
            unit = 'GB'
        if (number_of_bytes / step_to_greater_unit) >= 1:
            number_of_bytes /= step_to_greater_unit
            unit = 'TB'
        precision = 1
        number_of_bytes = round(number_of_bytes, precision)
        calculated_size = str(number_of_bytes) + ' ' + unit
        
        if file_size > size_limit:
            return "SIZE_ERROR"
        else:
            return "OK"