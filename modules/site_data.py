sites_data_dict = {
    "GoFile": {
        "apiKey": False,
        "url": "https://{server}.gofile.io/uploadFile",
        "api_url": "https://gofile.io/api/",
        "download_url_base": "https://gofile.io/d/",
        "server_url": "https://api.gofile.io/getServer"
    },
    "PixelDrain": {
        "apiKey": False,
        "url": "https://pixeldrain.com/api/file/",
        "api_url": "https://pixeldrain.com/api/",
        "download_url_base": "https://pixeldrain.com/u/",
        "size_limit": 20,
        "size_unit": "GB"
    },
    "FileBin": {
        "apiKey": False,
        "url": "https://filebin.net/",
        "api_url": "https://filebin.net/",
        "download_url_base": "https://filebin.net/"
    },
    "Delafil": {
        "apiKey": False,
        "url": "https://delafil.se/core/page/ajax/file_upload_handler.ajax.php",
        "api_url": "https://delafil.se/",
        "download_url_base": "https://delafil.se/",
        "size_limit": 6,
        "size_unit": "GB"
    },
    "Files.dp.ua": {
        "apiKey": False,
        "url": "https://files2.dp.ua/upload.php",
        "api_url": "https://files2.dp.ua/upload.php",
        "download_url_base": "https://files.dp.ua/en/",
        "size_limit": 100,
        "size_unit": "GB"
    },
    "FilesFM": {
        "apiKey": False,
        "url": "https://free.files.fm/save_file.php?up_id={upload_id}&ignore_user_abort=1&skip_update=1&key={upload_key}",
        "api_url": "https://free.files.fm/save_file.php",
        "download_url_base": "https://files.fm/u/{upload_id}",
        "initialize_url": "https://files.fm/server_scripts/get_upload_id.php?show_add_key=1",
        "finalize_url": "https://free.files.fm/finish_upload.php?upload_hash={upload_id}",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "Krakenfiles": {
        "apiKey": False,
        "url": "https://{server}.krakenfiles.com/api/file",
        "api_url": "https://krakenfiles.com/",
        "download_url_base": "https://krakenfiles.com",
        "server_url": "https://krakenfiles.com/api/server/available",
        "site_upload_url": "https://uploads{number}.krakenfiles.com/_uploader/gallery/upload",
        "size_limit": 1,
        "size_unit": "GB"
    },
    "Transfer": {
        "apiKey": False,
        "url": "https://transfer.sh/{file_name}",
        "api_url": "https://transfer.sh/",
        "download_url_base": "https://transfer.sh/",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "TmpFiles": {
        "apiKey": False,
        "url": "https://tmpfiles.org/api/v1/upload",
        "api_url": "https://tmpfiles.org/api",
        "download_url_base": "https://tmpfiles.org/",
        "size_limit": 100,
        "size_unit": "MB"
    },
    "Mixdrop": {
        "apiKey": False,
        "url": "https://ul.mixdrop.co/up",
        "api_url": "https://ul.mixdrop.co/up",
        "download_url_base": "https://mixdrop.co/f/",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "1Fichier": {
        "apiKey": False,
        "url": "https://{server}/upload.cgi?id={upload_id}",
        "api_url": "https://api.1fichier.com/v1/upload/",
        "download_url_base": "https://1fichier.com/?",
        "initialize_url": "https://api.1fichier.com/v1/upload/get_upload_server.cgi",
        "finalize_url": "https://{server}/end.pl?xid={upload_id}",
        "size_limit": 300,
        "size_unit": "GB"
    },
    "YourFileStore": {
        "apiKey": False,
        "url": "https://yourfilestore.com/upload",
        "api_url": "https://yourfilestore.com/upload",
        "download_url_base": "https://yourfilestore.com/download/",
        "size_limit": 500,
        "size_unit": "MB"
    },
    "Fileio": {
        "apiKey": False,
        "url": "https://file.io/?title={file_name}",
        "api_url": "https://file.io/",
        "download_url_base": "https://file.io/",
        "patch_url": "https://file.io/{file_id}",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "EasyUpload": {
        "apiKey": False,
        "url": "https://upload{number}.easyupload.io/action.php",
        "api_url": "https://easyupload.io/",
        "download_url_base": "https://easyupload.io/",
        "initialize_url": "https://easyupload.io/action.php",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "Oshi": {
        "comment": "Keep this one as low as possible. Its really a slow piece of shit.",
        "apiKey": False,
        "url": "https://oshi.at/?shorturl=1",
        "api_url": "https://oshi.at/cmd",
        "download_url_base": "https://oshi.at/",
        "size_limit": 5,
        "size_unit": "GB"
    },
}

UNITS = {
    'B': 1,
    'KB': 1024,
    'MB': 1024 * 1024,
    'GB': 1024 * 1024 * 1024,
    'TB': 1024 * 1024 * 1024 * 1024,
    'PB': 1024 * 1024 * 1024 * 1024 * 1024,
}

class Site_Data_CLSS:
    def size_unit_calc(site_name, file_size):
        site_data = sites_data_dict.get(site_name)
        if site_data is None:
            raise ValueError(f"Site '{site_name}' not found in the data dictionary.")

        size_limit = site_data.get("size_limit", 0)
        size_unit = site_data.get("size_unit", "B")
        
        size_limit_bytes = size_limit * UNITS.get(size_unit.upper(), 1)

        if file_size > size_limit_bytes:
            return "SIZE_ERROR"
        return "OK"
