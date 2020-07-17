

const { name, description } = require('../../package')

module.exports = {
    title: name,
    description: description,
    head: [
        ['meta', { name: 'theme-color', content: '#3eaf7c' }],
        ['meta', { name: 'apple-mobile-web-app-capable', content: 'yes' }],
        ['meta', { name: 'apple-mobile-web-app-status-bar-style', content: 'black' }]
    ],
    themeConfig: {
        docsRepo: 'wyhaya/see',
        docsDir: 'docs',
        editLinks: true,
        smoothScroll: true,
        lastUpdated: true,
        nav: [
            {
                text: 'Guide',
                link: '/',
            },
            {
                text: 'GitHub',
                link: 'https://github.com/wyhaya/see'
            }
        ],
        sidebar: {
            '/': [
                {
                    title: 'Guide',
                    collapsable: false,
                    children: [
                        '',
                        'install',
                        'start',
                        'config',
                        'var'
                    ]
                }
            ],
        }
    }
}
