module top;
  localparam K = 4;
  if (1) begin : gb
    function automatic integer f(); f = K; endfunction
    initial #1 $display("@f=%0d", f());
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
