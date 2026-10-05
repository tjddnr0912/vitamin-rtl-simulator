module top;
  function automatic logic [3:0] f(input int a);
    integer i;
    f = 4'd0;
    while (i < 3) begin f = f + 4'd1; i = i + 1; end
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
