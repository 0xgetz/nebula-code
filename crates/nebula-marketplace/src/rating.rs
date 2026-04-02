//! Rating and review system for skills in the marketplace
//!
//! This module provides types and functionality for rating skills (1-5 stars)
//! and writing reviews, as well as aggregating ratings for display and filtering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::types::Skill;

/// Errors that can occur in rating operations
#[derive(Error, Debug)]
pub enum RatingError {
    #[error("Invalid rating: must be between 1 and 5")]
    InvalidRating,
    #[error("Review error: {0}")]
    ReviewError(String),
}

/// A rating value from 1 to 5 stars
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Rating(u8);

impl Rating {
    /// Create a new rating (1-5 stars)
    ///
    /// # Errors
    /// Returns an error if the value is not between 1 and 5
    pub fn new(value: u8) -> Result<Self, RatingError> {
        if value < 1 || value > 5 {
            Err(RatingError::InvalidRating)
        } else {
            Ok(Self(value))
        }
    }

    /// Create a new rating without validation (for internal use)
    pub(crate) fn new_unchecked(value: u8) -> Self {
        Self(value)
    }

    /// Get the rating value
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Check if this is a perfect 5-star rating
    pub fn is_perfect(&self) -> bool {
        self.0 == 5
    }

    /// Check if this is a passing rating (3 stars or above)
    pub fn is_passing(&self) -> bool {
        self.0 >= 3
    }
}

impl TryFrom<u8> for Rating {
    type Error = RatingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::fmt::Display for Rating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} star{}", self.0, if self.0 == 1 { "" } else { "s" })
    }
}

/// A review of a skill, containing a rating and optional comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// The rating given (1-5 stars)
    pub rating: Rating,
    /// Optional comment text
    pub comment: Option<String>,
    /// Author of the review
    pub author: String,
    /// Timestamp of when the review was created (ISO 8601 format)
    pub timestamp: String,
}

impl Review {
    /// Create a new review with a rating and author
    pub fn new(rating: Rating, author: String, timestamp: String) -> Self {
        Self {
            rating,
            comment: None,
            author,
            timestamp,
        }
    }

    /// Add a comment to the review
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Create a review with a comment
    pub fn with_comment_full(
        rating: Rating,
        author: String,
        timestamp: String,
        comment: String,
    ) -> Self {
        Self {
            rating,
            comment: Some(comment),
            author,
            timestamp,
        }
    }
}

/// Aggregated rating statistics for a skill
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillRating {
    /// Average rating (0.0 if no ratings)
    pub average: f64,
    /// Total number of ratings
    pub count: u32,
    /// Distribution of ratings (1-star count, 2-star count, etc.)
    pub distribution: HashMap<u8, u32>,
    /// All reviews for this skill
    pub reviews: Vec<Review>,
}

impl SkillRating {
    /// Create a new empty skill rating
    pub fn new() -> Self {
        Self {
            average: 0.0,
            count: 0,
            distribution: HashMap::new(),
            reviews: Vec::new(),
        }
    }

    /// Add a rating to the aggregation
    pub fn add_rating(&mut self, rating: Rating) {
        let value = rating.value();
        *self.distribution.entry(value).or_insert(0) += 1;
        self.count += 1;
        self.recalculate_average();
    }

    /// Add a review (which includes a rating)
    pub fn add_review(&mut self, review: Review) {
        let rating = review.rating;
        let value = rating.value();
        *self.distribution.entry(value).or_insert(0) += 1;
        self.count += 1;
        self.reviews.push(review);
        self.recalculate_average();
    }

    /// Recalculate the average rating from the distribution
    fn recalculate_average(&mut self) {
        if self.count == 0 {
            self.average = 0.0;
            return;
        }

        let mut total: u64 = 0;
        for (stars, count) in &self.distribution {
            total += (*stars as u64) * (*count as u64);
        }
        self.average = total as f64 / self.count as f64;
    }

    /// Get the average rating rounded to one decimal place
    pub fn average_rounded(&self) -> f64 {
        (self.average * 10.0).round() / 10.0
    }

    /// Get the median rating
    pub fn median(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }

        let mut ratings = Vec::new();
        for (stars, count) in &self.distribution {
            for _ in 0..*count {
                ratings.push(*stars as f64);
            }
        }
        ratings.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mid = ratings.len() / 2;
        if ratings.len() % 2 == 0 {
            Some((ratings[mid - 1] + ratings[mid]) / 2.0)
        } else {
            Some(ratings[mid])
        }
    }

    /// Get the most common rating (mode)
    pub fn mode(&self) -> Option<u8> {
        self.distribution
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(stars, _)| *stars)
    }

    /// Check if the skill has any ratings
    pub fn has_ratings(&self) -> bool {
        self.count > 0
    }

    /// Get recent reviews (limited to n most recent)
    pub fn recent_reviews(&self, n: usize) -> Vec<&Review> {
        self.reviews.iter().rev().take(n).collect()
    }

    /// Get all reviews with at least a certain rating
    pub fn reviews_above(&self, min_rating: u8) -> Vec<&Review> {
        self.reviews
            .iter()
            .filter(|r| r.rating.value() >= min_rating)
            .collect()
    }
}

/// Extension trait for adding rating functionality to Skill
pub trait SkillRatable {
    /// Get the rating for this skill
    fn rating(&self) -> Option<&SkillRating>;

    /// Get a mutable reference to the rating for this skill
    fn rating_mut(&mut self) -> Option<&mut SkillRating>;

    /// Check if the skill has ratings
    fn has_ratings(&self) -> bool {
        self.rating().map_or(false, |r| r.has_ratings())
    }

    /// Get the average rating, or 0.0 if no ratings
    fn average_rating(&self) -> f64 {
        self.rating().map_or(0.0, |r| r.average)
    }
}

/// Extension trait for the registry to support rating queries
pub trait RatingQuery {
    /// Find skills with average rating >= min_rating
    fn find_by_min_rating(&self, min_rating: f64) -> Vec<&Skill>;

    /// Find skills with average rating >= min_rating and at least min_reviews reviews
    fn find_by_rating_and_reviews(&self, min_rating: f64, min_reviews: u32) -> Vec<&Skill>;

    /// Get the top N rated skills
    fn top_rated(&self, n: usize) -> Vec<&Skill>;

    /// Get skills that have no ratings yet
    fn find_unrated(&self) -> Vec<&Skill>;

    /// Get skills with at least one review containing a comment
    fn find_with_reviews(&self) -> Vec<&Skill>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_creation() {
        assert!(Rating::new(1).is_ok());
        assert!(Rating::new(5).is_ok());
        assert!(Rating::new(0).is_err());
        assert!(Rating::new(6).is_err());
    }

    #[test]
    fn test_rating_value() {
        let rating = Rating::new(4).unwrap();
        assert_eq!(rating.value(), 4);
        assert!(!rating.is_perfect());
        assert!(rating.is_passing());
    }

    #[test]
    fn test_rating_display() {
        let one_star = Rating::new(1).unwrap();
        assert_eq!(one_star.to_string(), "1 star");

        let five_stars = Rating::new(5).unwrap();
        assert_eq!(five_stars.to_string(), "5 stars");
    }

    #[test]
    fn test_review_creation() {
        let review = Review::new(Rating::new(5).unwrap(), "author".to_string(), "2024-01-01T00:00:00Z".to_string());
        assert_eq!(review.rating.value(), 5);
        assert_eq!(review.author, "author");
        assert!(review.comment.is_none());

        let review_with_comment = review
            .with_comment("Great skill!".to_string());
        assert_eq!(review_with_comment.comment, Some("Great skill!".to_string()));
    }

    #[test]
    fn test_skill_rating_aggregation() {
        let mut rating = SkillRating::new();

        rating.add_rating(Rating::new(5).unwrap());
        rating.add_rating(Rating::new(4).unwrap());
        rating.add_rating(Rating::new(5).unwrap());

        assert_eq!(rating.count, 3);
        assert!((rating.average - 4.666666).abs() < 0.001);
        assert_eq!(rating.distribution.get(&5), Some(&2));
        assert_eq!(rating.distribution.get(&4), Some(&1));
    }

    #[test]
    fn test_skill_rating_with_reviews() {
        let mut rating = SkillRating::new();

        let review1 = Review::new(Rating::new(5).unwrap(), "alice".to_string(), "2024-01-01T00:00:00Z".to_string())
            .with_comment("Excellent!".to_string());
        let review2 = Review::new(Rating::new(3).unwrap(), "bob".to_string(), "2024-01-02T00:00:00Z".to_string());

        rating.add_review(review1);
        rating.add_review(review2);

        assert_eq!(rating.count, 2);
        assert_eq!(rating.average, 4.0);
        assert_eq!(rating.reviews.len(), 2);
        assert_eq!(rating.recent_reviews(1).len(), 1);
        assert_eq!(rating.reviews_above(4).len(), 1);
    }

    #[test]
    fn test_skill_rating_median_and_mode() {
        let mut rating = SkillRating::new();
        rating.add_rating(Rating::new(5).unwrap());
        rating.add_rating(Rating::new(3).unwrap());
        rating.add_rating(Rating::new(4).unwrap());
        rating.add_rating(Rating::new(5).unwrap());

        assert_eq!(rating.median(), Some(4.5));
        assert_eq!(rating.mode(), Some(5));
    }

    #[test]
    fn test_skill_rating_empty() {
        let rating = SkillRating::new();
        assert_eq!(rating.average, 0.0);
        assert_eq!(rating.count, 0);
        assert!(!rating.has_ratings());
        assert_eq!(rating.median(), None);
        assert_eq!(rating.mode(), None);
    }

    #[test]
    fn test_rating_try_from() {
        let rating: Result<Rating, _> = 4u8.try_into();
        assert!(rating.is_ok());
        assert_eq!(rating.unwrap().value(), 4);

        let bad: Result<Rating, _> = 0u8.try_into();
        assert!(bad.is_err());
    }
}

/// Implement SkillRatable for Skill
impl SkillRatable for Skill {
    fn rating(&self) -> Option<&SkillRating> {
        self.rating.as_ref()
    }

    fn rating_mut(&mut self) -> Option<&mut SkillRating> {
        self.rating.as_mut()
    }
}
