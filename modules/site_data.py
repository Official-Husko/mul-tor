sites_data_dict = {
    "pixeldrain": {
        "apiKey": False,
        "url": "https://pixeldrain.com/api/file/{name}",
        "size_limit": 20,
        "size_unit": "GB"
    }
}

class Site_Data_CLSS:
    
    def size_unit_calc(site_name, file_size):
        site_name = site_name.lower()
        size_unit = sites_data_dict[site_name.lower()]["size_unit"]
        size_limit = sites_data_dict[site_name.lower()]["size_limit"]
        
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
        
        if calculated_size > size_limit:
            return "SIZE_ERROR"
        else:
            return "OK"