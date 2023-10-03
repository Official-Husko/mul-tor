# Mul-Tor
## Your tool for easy file uploading and sharing.

#### Like mentioned above the goal of this tool is to help people upload files fast to multiple hosters while also helping in the process of sharing these links.

<br />

### Preview

![preview](https://github.com/Official-Husko/mul-tor/blob/master/media/preview.gif)  

<br />

[**Download it here**](https://github.com/Official-Husko/mul-tor/releases/latest)

<br />

### Features:

* Proxies
* Random User Agent
* Check Website Availability
* Progress Bar
* Auto Updater
* Site Presets (Coming with 1.3.0)

<br />

### Currently supported sites:
Site | API | Api Key Required | Max File Size
--- | --- | --- | ---
[GoFile][7] | [Yes][8] | No | ∞
[PixelDrain][1] | [Yes][2] | No | 20 GB
[Filebin][92] | No | No | ∞
[Delafil][107] | No | No | 6 GB
[Files.dp.ua][108] | No | No | 100 GB
[Files.fm][45] | No | No | 5 GB
[Krakenfiles][124] | No | No | 1 GB
[Transfer.sh][98] | No | No | ∞
[/tmp/files][117] | [Yes][118] | No | 100 MB
[Mixdrop][29] | No | No | ∞
[1Fichier][31] | [Yes][32] | No | 300 GB
[YourFileStore][123] | No | No | 500 MB
[Oshi][9] | No | No | 5 GB
[File.io][70] | No | No | 2 GB
[EasyUpload][72] | No | No | 10 GB

<br />

### Planned Sites:
Site | API | Api Key Required | Max File Size
--- | --- | --- | ---
[BowFile][17] | [Yes][18] | [Yes][19] | 5 GB
[1CloudFile][20] | [Yes][21] | [Yes][22] | 5 GB
[SendSpace][26] | [Yes][27] | [Yes][28] | 300 MB
[Doodrive][36] | [Yes][37] | [Yes][38] | 5 GB
[FastUpload][67] | No | No | 10 GB
[UFile][68] | [Yes][69] | No | 5 GB

<br />

### Rejected Sites:
<details>

  Site | API | Api Key Required | Max File Size | Reason
  --- | --- | --- | --- | ---
  [DropMeFiles][106] | No | No | 50 GB | Terrible Uploading System
  [Up2Share][120] | No | No | 1 GB | Terrible Uploading System
  [WeTransfer][121] | No | No | 2 GB | When do the terrible uploading systems end?
  [Filemail][42] | [Yes][43] | [No/Yes*²][44] | 5 GB | Garbage Limit of 2 Uploads a Day
  [MEGA][109] | [Yes][110] | No | 20 GB | I just can't be bothered
  [Google Drive][111] | [Yes][112] | No | 15 GB | Same as MEGA
  [Mediafire][113] | [Yes][114] | No | 10 GB | Same as MEGA
  [UploadHaven][115] | No | No | 50 GB | Paid/Invite Only
  [Terminal][116] | No | No | ? | Invite Only
  [Uptobox][33] | [Yes][34] | [Yes][35] | 200 GB | Raided by the Feds/ACE & Down
  [SendGB][119] | No | No | 5 GB | Links are funky and garbage upload system
  [WorkUpload][122] | No | No | 2 GB | Gives a 200 response with a link but the file is not available.
  [RocketFile][125] | No | No | ? | I don't know what the fuck this is but no
  [Qiwi.gg][126] | No | No | ? | Really Complicated system
  [CyberFiles][128] | No | No | 19 GB | sometimes throws account missing errors and sometimes it uploads fine
  [send.zcyph.cc][129] | No | No | ? | I have never seen or heard of an upload system like this
  [SendSpace][130] | No | No | 300 MB | SSL issues and terrible upload system

</details>

<br />

### Q&A
<details>
  <summary>Open Q&A</summary>
  Q: Where did the sites like Anonfiles.com go?

  A: Anonfiles.com threw in the towel. With that the mirrors are also gone.

  Q: How do I get the API key?\
  A: Click on the blue yes in the Api Key Required row for the site you wish to get a key for.

  Q: I want to request a site.\
  A: Please first check the [rejected sites][3] and [issues page][4] to ensure it hasn't been previously mentioned. If not, feel free to open a new issue.

  Q: What about rejected sites?\
  A: I will check the sites every once in a while to see if the issues i mentioned have been resolved. If you know the issue has been resolved or why i encountered an issue feel free to open a new issue.

  Q: Why is it called Mul-Tor?\
  A: I have no idea.

  Q: What if i really need one of the rejected sites?\
  A: This project is open to contributions. Maybe somebody else can add it. You may open a new issue if there isn't one already so that i can check it again.

  Q: Certain sites offer an API but you didn't use it. Why?\
  A: Because this makes it easier for users and it doesn't need any accounts and sign ups. I plan to add api functions to these for users that want to use it.
</details>

[comment]: # (Below are all links to the sites)
[0]: #
[1]: https://pixeldrain.com/
[2]: https://pixeldrain.com/api
[3]: https://github.com/Official-Husko/mul-tor#rejected-sites
[4]: https://github.com/Official-Husko/mul-tor/issues
[7]: https://gofile.io/
[8]: https://gofile.io/api
[9]: https://oshi.at/
[17]: https://bowfile.com/
[18]: https://bowfile.com/api
[19]: https://bowfile.com/account/edit#api
[20]: https://1cloudfile.com/
[21]: https://1cloudfile.com/api
[22]: https://1cloudfile.com/account/edit#api
[23]: https://hexupload.net/
[24]: https://hexupload.docs.apiary.io/#
[25]: https://hexupload.net/?op=my_account
[26]: https://sendspace.com/
[27]: https://sendspace.com/dev_method.html
[28]: https://sendspace.com/dev_apikeys.html
[29]: https://mixdrop.co/
[30]: https://mixdrop.co/api/
[31]: https://1fichier.com/
[32]: https://1fichier.com/api.html
[33]: https://uptobox.com/
[34]: https://docs.uptobox.com/
[35]: https://uptobox.com/my_account
[36]: https://doodrive.com/
[37]: https://doodrive.com/dashboard/api
[38]: https://doodrive.com/dashboard/settings#settings_api
[39]: https://transfernow.net/
[40]: https://developers.transfernow.net/
[41]: https://transfernow.net/dashboard/admin/api
[42]: https://filemail.com/
[43]: https://filemail.com/apidoc
[44]: https://filemail.com/apidoc/ApiKey.aspx
[45]: https://files.fm/
[46]: https://files.fm/api.php
[67]: https://fastupload.io/
[68]: https://ufile.io/
[69]: https://ufile.io/blog/?filter=api
[70]: https://file.io/
[72]: https://easyupload.io/
[73]: https://filepost.io/
[74]: https://file-upload.net/
[75]: https://transferxl.com/
[76]: https://filetransfer.io/
[77]: https://transfernow.net/
[78]: https://quicklyupload.com/
[79]: https://tmpfiles.org/
[80]: https://send.vis.ee/
[81]: https://wormhole.app/
[82]: https://swisstransfer.com/en-us
[83]: https://4shared.com/
[84]: https://instant.io/
[85]: https://send.tresorit.com/
[86]: https://sharrr.com/
[87]: https://blackhole.run/
[88]: https://filedropper.com/
[89]: https://myairbridge.com/en/#!/settings
[90]: https://sendgb.com/
[91]: https://ulozto.net/
[92]: https://filebin.net/
[93]: https://send.cm/
[94]: https://filewhopper.com/
[95]: https://transferfile.io/#/
[96]: https://hotdropp.com/#/
[97]: https://cyberdrop.me/
[98]: https://transfer.sh/
[99]: https://k2s.cc/
[100]: https://rapidgator.net/
[101]: https://1fichier.com/
[102]: https://download.gg/
[103]: https://torrentfreak.com/file-hosting-icon-anonfiles-throws-in-the-towel-domain-for-sale-230817/
[104]: https://anonfiles.me/
[105]: https://anonfiles.me/docs/api
[106]: https://dropmefiles.com/
[107]: https://delafil.se/
[108]: https://files.dp.ua/
[109]: https://mega.nz/
[110]: https://mega.io/developers
[111]: https://www.google.com/drive/
[112]: https://developers.google.com/drive/api/guides/about-sdk
[113]: https://www.mediafire.com/
[114]: https://www.mediafire.com/developers/core_api/1.5/getting_started/
[115]: https://uploadhaven.com/
[116]: https://terminal.lc/
[117]: https://tmpfiles.org/
[118]: https://tmpfiles.org/api
[119]: https://www.sendgb.com/
[120]: https://up2sha.re/
[121]: https://wetransfer.com/
[122]: https://workupload.com/
[123]: http://yourfilestore.com/
[124]: https://krakenfiles.com/
[125]: https://rocketfile.co/
[126]: https://qiwi.gg/
[127]: https://anontransfer.com/
[128]: https://cyberfile.me/
[129]: https://send.zcyph.cc/
[130]: https://sendspace.com/
[131]: https://anonfiles.me/
[132]: https://anonfiles.me/docs/api

<br />

#### Disclaimer
*I am not in any way affiliated with or working with these sites. ***This is an unofficial project***. I am not responsible and or liable for the content that is uploaded with this tool.*