module top;
  function automatic integer f(input integer a); $display("hi"); f = a * 2; endfunction
  if (1) begin : gb
    localparam P = f(3);
    initial #1 $display("@P=%0d", P);
  end
  initial #5 $finish;
endmodule
