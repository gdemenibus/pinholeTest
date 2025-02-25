use cgmath::BaseNum;
use cgmath::Matrix4;
use cgmath::Vector3;
use cgmath::Vector4;

pub trait ToArr {
    type Output;
    fn to_arr(&self) -> Self::Output;
}

pub trait FromArr {
    type Input;
    fn from_arr(array: Self::Input) -> Self;

}

impl<T: BaseNum> ToArr for Matrix4<T> {
    type Output = [[T; 4]; 4];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}



// Go back to array
impl<T: BaseNum> ToArr for Vector3<T> {
    type Output = [T; 3];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}
// Create from array
impl<T:BaseNum> FromArr for Vector3<T> {
    type Input = [T; 3];
    fn from_arr(array: Self::Input) -> Vector3<T>{
        Vector3::new(array[0], array[1], array[2])

    }
    
}

// Go back to array
impl<T: BaseNum> ToArr for Vector4<T> {
    type Output = [T; 4];
    fn to_arr(&self) -> Self::Output {
        (*self).into()
    }
}
// Create from array
impl<T:BaseNum> FromArr for Vector4<T> {
    type Input = [T; 4];
    fn from_arr(array: Self::Input) -> Vector4<T>{
        Vector4::new(array[0], array[1], array[2], array[3])

    }
    
}
