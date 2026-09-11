'use strict';
/**
 * PressRoom seed world — media/publishing (editorial, assets, approvals,
 * publication/distribution, corrections, social). Synthetic data only.
 */

function user(id, username, name, role, permissions, tokenParts, passwordParts) {
  return { id, username, name, role, permissions, tokenParts, passwordParts };
}

function build() {
  return {
    users: [
      user('u-1', 'camille.dubois', 'Camille Dubois', 'editor_in_chief',
        ['story:read', 'story:create', 'story:edit', 'story:approve', 'story:publish', 'correction:apply', 'channel:read', 'asset:read'],
        ['pr', 'eic', 'camille', '9a61'], ['demo', 'pass', 'camille']),
      user('u-2', 'diego.morales', 'Diego Morales', 'section_editor',
        ['story:read', 'story:edit', 'story:approve', 'story:publish', 'correction:apply', 'channel:read', 'asset:read'],
        ['pr', 'edt', 'diego', '4b27'], ['demo', 'pass', 'diego']),
      user('u-3', 'hana.kim', 'Hana Kim', 'staff_writer',
        ['story:read', 'story:create', 'story:edit', 'asset:read', 'channel:read'],
        ['pr', 'wrt', 'hana', 'c8e0'], ['demo', 'pass', 'hana']),
      user('u-4', 'omar.bhatt', 'Omar Bhatt', 'photo_editor',
        ['story:read', 'asset:read', 'asset:upload', 'rendition:request'],
        ['pr', 'pho', 'omar', '1d34'], ['demo', 'pass', 'omar']),
      user('u-5', 'nia.roberts', 'Nia Roberts', 'social_distribution_manager',
        ['story:read', 'social:schedule', 'social:post', 'channel:read', 'asset:read'],
        ['pr', 'soc', 'nia', '77f5'], ['demo', 'pass', 'nia']),
    ],
    sections: ['news', 'business', 'culture', 'sports', 'metro'],
    channels: [
      { id: 'ch-1', name: 'web-front-page', kind: 'web', licenseValidUntil: null },
      { id: 'ch-2', name: 'morning-newsletter', kind: 'email', licenseValidUntil: null },
      { id: 'ch-3', name: 'rss-feed', kind: 'syndication', licenseValidUntil: null },
      { id: 'ch-4', name: 'regional-syndication', kind: 'syndication', licenseValidUntil: '2027-06-30' },
      { id: 'ch-5', name: 'partner-app', kind: 'partner', licenseValidUntil: '2026-06-01' },
    ],
    assets: [
      { id: 'a-0301', kind: 'image', name: 'harbor-bridge-aerial.jpg', sizeMb: 4.2, width: 4096, height: 2304, credit: 'City Photo Desk', license: 'editorial',
        renditions: [{ name: 'thumbnail', dimensions: '320w', status: 'ready' }, { name: 'social-card', dimensions: '1200x630', status: 'ready' }, { name: 'hero', dimensions: '2560w', status: 'ready' }],
        checksum: 'aa12fe31' },
      { id: 'a-0302', kind: 'image', name: 'budget-hearing-chamber.jpg', sizeMb: 3.1, width: 3840, height: 2560, credit: 'Staff', license: 'editorial',
        renditions: [{ name: 'thumbnail', dimensions: '320w', status: 'ready' }], checksum: 'bb89dc04' },
      { id: 'a-0303', kind: 'image', name: 'ferry-terminal-render.png', sizeMb: 2.4, width: 3000, height: 2000, credit: 'Port Authority (handout)', license: 'editorial',
        renditions: [{ name: 'hero', dimensions: '2560w', status: 'ready' }], checksum: 'cc41ab77' },
      { id: 'a-0304', kind: 'video', name: 'terminal-timelapse.mp4', sizeMb: 82, width: 1920, height: 1080, credit: 'City Photo Desk', license: 'editorial',
        renditions: [{ name: 'web-1080p', dimensions: '1920x1080', status: 'processing' }], checksum: 'dd0753e9' },
    ],
    stories: [
      { id: 'st-0201', slug: 'harbor-bridge-retrofit-begins', title: 'Harbor bridge retrofit begins Monday', section: 'news', authorId: 'u-3',
        dek: 'Two-year closure plan for the upper deck starts with night work.',
        body: 'The long-planned retrofit of the Harbor Bridge begins Monday night, with lane closures on the upper deck from 22:00. Engineers say the deck truss needs replacing after inspection reports flagged corrosion in 2024.\n\nCyclists will keep a widened shared path for the first phase; bus routes 12 and 44 will shift to the tunnel during night work.',
        status: 'published', version: 3, expectedVersion: 3, wordCount: 210,
        assets: [{ assetId: 'a-0301', role: 'hero' }], channels: ['ch-1', 'ch-2', 'ch-3'],
        submittedAt: '2026-09-09T10:00:00Z', publishedAt: '2026-09-10T06:30:00Z', approvedById: 'u-1',
        socialPosts: ['sp-0101', 'sp-0102'] },
      { id: 'st-0202', slug: 'city-budget-hearings-five-things', title: 'City budget hearings: five things to watch', section: 'metro', authorId: 'u-3',
        dek: 'Housing, transit and the library levy dominate a long agenda.',
        body: 'Five storylines will shape this month’s budget hearings: the housing trust top-up, transit fare freezes, the library levy, stormwater fees, and a contested police overtime line.\n\nEach has a bloc of council allies and a fiscal note that will be picked apart line by line.',
        status: 'in_review', version: 2, expectedVersion: 2, wordCount: 168,
        assets: [{ assetId: 'a-0302', role: 'inline' }], channels: [],
        submittedAt: '2026-09-11T14:15:00Z', publishedAt: null, approvedById: null, socialPosts: [] },
      { id: 'st-0203', slug: 'new-ferry-terminal-designs', title: 'New ferry terminal designs unveiled', section: 'news', authorId: 'u-3',
        dek: 'Three concepts, one very public fight about sightlines.',
        body: 'Draft: The port authority released three design concepts for the downtown ferry terminal...',
        status: 'draft', version: 1, expectedVersion: 1, wordCount: 42,
        assets: [], channels: [], submittedAt: null, publishedAt: null, approvedById: null, socialPosts: [] },
      { id: 'st-0204', slug: 'marathon-route-changes', title: 'Marathon route changes draw mixed reviews', section: 'sports', authorId: 'u-3',
        dek: 'Runners like the flatter course; downtown merchants do not.',
        body: 'The marathon’s new route trades the hill district for two downtown laps, shaving an estimated 4 minutes for mid-pack runners but closing Saturday morning access for roughly forty businesses.',
        status: 'approved', version: 2, expectedVersion: 2, wordCount: 96,
        assets: [{ assetId: 'a-0303', role: 'hero' }], channels: [],
        submittedAt: '2026-09-10T09:30:00Z', publishedAt: null, approvedById: 'u-1', socialPosts: [] },
      { id: 'st-0205', slug: 'council-stadium-funding-vote', title: 'Council votes on stadium funding tonight', section: 'metro', authorId: 'u-3',
        dek: 'A late amendment could delay the vote again.',
        body: 'The council meets at 19:00 to vote on the stadium funding package. A late amendment would move maintenance costs to the general fund.',
        status: 'approved', version: 1, expectedVersion: 1, wordCount: 34,
        assets: [], channels: [], submittedAt: '2026-09-11T16:00:00Z', publishedAt: null, approvedById: 'u-1', socialPosts: [] },

    ],
    distributionJobs: [
      { id: 'dj-0601', storyId: 'st-0201', channelId: 'ch-1', status: 'sent', at: '2026-09-10T06:30:00Z', error: null },
      { id: 'dj-0602', storyId: 'st-0201', channelId: 'ch-2', status: 'sent', at: '2026-09-10T06:31:00Z', error: null },
      { id: 'dj-0603', storyId: 'st-0201', channelId: 'ch-3', status: 'sent', at: '2026-09-10T06:31:00Z', error: null },
    ],
    socialPosts: [
      { id: 'sp-0101', storyId: 'st-0201', network: 'feedbook', text: 'The Harbor Bridge retrofit starts Monday night — what it means for your commute.',
        status: 'posted', scheduledAt: '2026-09-10T07:00:00Z', postedAt: '2026-09-10T07:00:00Z', engagement: { likes: 842, reposts: 61, clicks: 3102 } },
      { id: 'sp-0102', storyId: 'st-0201', network: 'chirp', text: 'Upper deck closes nightly at 22:00 starting Monday. Cyclists keep the path.',
        status: 'scheduled', scheduledAt: '2026-09-13T08:00:00Z', postedAt: null, engagement: null },
    ],
    corrections: [
      { id: 'c-0801', storyId: 'st-0201', summary: 'Fixed bus route numbers (12 and 44, not 14 and 42)',
        beforeText: 'bus routes 14 and 42', afterText: 'bus routes 12 and 44', byId: 'u-2', at: '2026-09-10T11:05:00Z', status: 'published' },
    ],
    notifications: [
      { id: 'n-1', userId: 'u-3', to: 'hana.kim', type: 'editorial', subject: 'Your story st-0204 was approved',
        text: 'Camille Dubois approved "Marathon route changes draw mixed reviews" for publication.', status: 'delivered', createdAt: '2026-09-10T10:02:00Z' },
    ],
    events: [],
    meta: { today: '2026-09-12', storySeq: 205, assetSeq: 304, jobSeq: 603, socialSeq: 102, corrSeq: 801 },
  };
}

module.exports = { build };
