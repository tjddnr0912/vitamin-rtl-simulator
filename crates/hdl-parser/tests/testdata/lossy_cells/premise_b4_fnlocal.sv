module t;
  function automatic logic [7:0] f();
    localparam unsigned U = 4'sd8;
    localparam logic [3:0] L = 4'b1x01;
    return {U > 0, 3'b000, L};
  endfunction
  initial begin #1 $display("A f=%b", f()); end
  initial #100 $finish;
endmodule
