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
        "api_url": "https://api.anonfiles.com/",
        "download_url_base": "https://anonfiles.com/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "BayFiles": {
        "apiKey": False,
        "url": "https://api.bayfiles.com/upload",
        "api_url": "https://api.bayfiles.com/",
        "download_url_base": "https://bayfiles.com/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "OpenLoad": {
        "apiKey": False,
        "url": "https://api.openload.cc/upload",
        "api_url": "https://api.openload.cc/",
        "download_url_base": "https://openload.cc/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "LolaBits": {
        "apiKey": False,
        "url": "https://api.lolabits.se/upload",
        "api_url": "https://api.lolabits.se/",
        "download_url_base": "https://lolabits.se/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "vShare": {
        "apiKey": False,
        "url": "https://api.vshare.is/upload",
        "api_url": "https://api.vshare.is/",
        "download_url_base": "https://vshare.is/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "HotFile": {
        "apiKey": False,
        "url": "https://api.hotfile.io/upload",
        "api_url": "https://api.hotfile.io/",
        "download_url_base": "https://hotfile.io/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "RapidShare": {
        "apiKey": False,
        "url": "https://api.rapidshare.nu/upload",
        "api_url": "https://api.rapidshare.nu/",
        "download_url_base": "https://rapidshare.nu/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "UpVid": {
        "apiKey": False,
        "url": "https://api.upvid.cc/upload",
        "api_url": "https://api.upvid.cc/",
        "download_url_base": "https://upvid.cc/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "LetsUpload": {
        "apiKey": False,
        "url": "https://api.letsupload.cc/upload",
        "api_url": "https://api.letsupload.cc/",
        "download_url_base": "https://letsupload.cc/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "ShareOnline": {
        "apiKey": False,
        "url": "https://api.share-online.is/upload",
        "api_url": "https://api.share-online.is/",
        "download_url_base": "https://share-online.is/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "MegaUpload": {
        "apiKey": False,
        "url": "https://api.megaupload.nz/upload",
        "api_url": "https://api.megaupload.nz/",
        "download_url_base": "https://megaupload.nz/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "MyFile": {
        "apiKey": False,
        "url": "https://api.myfile.is/upload",
        "api_url": "https://api.myfile.is/",
        "download_url_base": "https://myfile.is/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "FileChan": {
        "apiKey": False,
        "url": "https://api.filechan.org/upload",
        "api_url": "https://api.filechan.org/",
        "download_url_base": "https://filechan.org/",
        "size_limit_human": 20,
        "size_limit_bytes": 21474836480,
        "size_unit": "GB"
    },
    "Oshi": {
        "apiKey": False,
        "url": "https://oshi.at/?shorturl=1",
        "api_url": "https://oshi.at/cmd",
        "download_url_base": "https://oshi.at/",
        "size_limit_human": 5,
        "size_limit_bytes": 5368709120,
        "size_unit": "GB"
    },
    "MixDrop": {
        "apiKey": True,
        "url": "https://ul.mixdrop.co/api",
        "api_url": "https://mixdrop.co/api",
        "download_url_base": "https://mixdrop.co/f/"
    },
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