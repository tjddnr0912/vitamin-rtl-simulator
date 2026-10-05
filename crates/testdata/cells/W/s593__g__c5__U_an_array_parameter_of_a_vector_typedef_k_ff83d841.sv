module tb; typedef logic [5:0] T;
  localparam T A[2] = '{6'd1, 9'h1FF};
  initial begin $display("DIGEST=%0d %0d %0d", A[0], A[1], $bits(A[1])); #1 $finish; end
endmodule