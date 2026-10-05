module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    integer i;
    if (a == 1) t = 4'd5;
    f = 4'd0;
    i = 0;
    while (t < 4'd3 && i < 5) begin f = f + 4'd1; i = i + 1; end
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
