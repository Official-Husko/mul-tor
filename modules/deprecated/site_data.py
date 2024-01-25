"""

All of the data in this dictionary is deprecated since the sites are down or the support has been removed.
They are kept here for reference and potential use in the future.

"""

sites_data_dict = {
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
    "Up2Share": {
        "apiKey": False,
        "url": "https://up2sha.re/upload",
        "api_url": "https://up2sha.re/",
        "download_url_base": "https://up2sha.re/file?f=",
        "size_limit": 1,
        "size_unit": "GB"
    },
    "WeTransfer": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://wetransfer.com/",
        "download_url_base": "https://wetransfer.com/downloads/{file_id}/{security_hash}",
        "announce_url": "https://wetransfer.com/api/v4/transfers/link",
        "initialize_url": "https://storm-eu-west-1.wetransfer.net/api/v2/batch/preflight",
        "prepare_blocks_url": "https://storm-eu-west-1.wetransfer.net/api/v2/blocks",
        "check_status_url": "https://storm-eu-west-1.wetransfer.net/api/v2/batch",
        "finalize_url": "https://wetransfer.com/api/v4/transfers/{file_id}/finalize",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "CyberFile": {
        "apiKey": False,
        "url": "https://cyberfile.me/ajax/file_upload_handler",
        "api_url": "https://cyberfile.me/",
        "download_url_base": "https://cyberfile.me/",
        "size_limit": 19,
        "size_unit": "GB"
    },
    "FileTransfer": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://filetransfer.io/",
        "download_url_base": "https://filetransfer.io/data-package/",
        "server_url": "https://filetransfer.io/api/v1/upload",
        "initialize_url": "https://filetransfer.io/start-upload",
        "finalize_url": "{final_url}",
        "size_limit": 6,
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
    "AnonFilesMe": {
        "apiKey": False,
        "url": "https://anonfiles.me/api/v1/upload",
        "api_url": "https://anonfiles.me/",
        "download_url_base": "https://anonfiles.me/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "NitroFile": {
        "apiKey": False,
        "url": "https://nitrofile.cc/api/v1/upload",
        "api_url": "https://nitrofile.cc/",
        "download_url_base": "https://nitrofile.cc/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "GoFileCC": {
        "apiKey": False,
        "url": "https://gofile.cc/api/v1/upload",
        "api_url": "https://gofile.cc/",
        "download_url_base": "https://gofile.cc/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "AnyFile": {
        "apiKey": False,
        "url": "https://anyfile.co/api/v1/upload",
        "api_url": "https://anyfile.co/",
        "download_url_base": "https://anyfile.co/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "BayFilesIo": {
        "apiKey": False,
        "url": "https://bayfiles.io/api/v1/upload",
        "api_url": "https://bayfiles.io/",
        "download_url_base": "https://bayfiles.io/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "Bunkrr": {
        "apiKey": True,
        "url": "{server}",
        "api_url": "https://app.bunkrr.su/",
        "download_url_base": "https://bunkrr.ru/d/",
        "server_url": "https://app.bunkrr.su/api/node",
        "options_url": "https://app.bunkrr.su/api/check",
        "finalize_url": "{server}/finishchunks"
    },
}