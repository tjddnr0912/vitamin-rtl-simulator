module top;
  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  function automatic integer fi(input int a);
    if (a == 1) fi = 10;
  endfunction
  function automatic logic signed [5:0] fs(input int a);
    fs = -6'sd3;
    if (a == 1) fs = 6'sd10;
  endfunction
  parameter PX = fx(2);
  parameter PI = fi(2);
  parameter PS = fs(2);
  initial begin #1 $display("PX=%b bx=%0d PI=%0d bi=%0d PS=%0d bs=%0d neg=%0d", PX, $bits(PX), PI, $bits(PI), PS, $bits(PS), PS < 0); $finish; end
endmodule
