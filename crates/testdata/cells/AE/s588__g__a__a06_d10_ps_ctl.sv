module top;
  function automatic logic signed [5:0] fs(input int a);
    fs = -6'sd3;
    if (a == 1) fs = 6'sd10;
  endfunction
  parameter PS = fs(2);
  initial begin #1 $display("PS=%0d bs=%0d neg=%0d", PS, $bits(PS), PS < 0); $finish; end
  initial #100 $finish;
endmodule
