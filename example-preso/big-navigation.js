// Simple slide navigation script for big-slides

window.addEventListener('DOMContentLoaded', function() {
  const slides = document.querySelectorAll('.slides > div');
  let currentSlide = 0;
  
  // Hide all slides except the first one
  function setupSlides() {
    slides.forEach((slide, index) => {
      slide.style.display = index === currentSlide ? 'flex' : 'none';
    });
    
    // Add slide number indicator
    const slideCount = document.createElement('div');
    slideCount.className = 'slide-count';
    slideCount.style.position = 'fixed';
    slideCount.style.bottom = '10px';
    slideCount.style.right = '10px';
    slideCount.style.background = 'rgba(0,0,0,0.5)';
    slideCount.style.padding = '5px 10px';
    slideCount.style.borderRadius = '3px';
    slideCount.style.fontSize = '14px';
    document.body.appendChild(slideCount);
    
    updateSlideCount();
  }
  
  function updateSlideCount() {
    const slideCount = document.querySelector('.slide-count');
    if (slideCount) {
      slideCount.textContent = `${currentSlide + 1} / ${slides.length}`;
    }
  }
  
  function nextSlide() {
    if (currentSlide < slides.length - 1) {
      slides[currentSlide].style.display = 'none';
      currentSlide++;
      slides[currentSlide].style.display = 'flex';
      updateSlideCount();
    }
  }
  
  function prevSlide() {
    if (currentSlide > 0) {
      slides[currentSlide].style.display = 'none';
      currentSlide--;
      slides[currentSlide].style.display = 'flex';
      updateSlideCount();
    }
  }
  
  // Keyboard navigation
  document.addEventListener('keydown', function(e) {
    if (e.key === 'ArrowRight' || e.key === ' ' || e.key === 'n') {
      nextSlide();
    } else if (e.key === 'ArrowLeft' || e.key === 'p') {
      prevSlide();
    }
  });
  
  // Click navigation
  document.addEventListener('click', function() {
    nextSlide();
  });
  
  // Touch navigation
  let touchStartX = 0;
  let touchEndX = 0;
  
  document.addEventListener('touchstart', function(e) {
    touchStartX = e.changedTouches[0].screenX;
  });
  
  document.addEventListener('touchend', function(e) {
    touchEndX = e.changedTouches[0].screenX;
    handleSwipe();
  });
  
  function handleSwipe() {
    if (touchEndX < touchStartX - 50) {
      nextSlide(); // Swipe left, go to next slide
    } else if (touchEndX > touchStartX + 50) {
      prevSlide(); // Swipe right, go to previous slide
    }
  }
  
  // Initialize slides
  setupSlides();
});