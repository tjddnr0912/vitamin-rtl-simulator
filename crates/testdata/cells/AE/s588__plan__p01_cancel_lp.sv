module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    t[0] = 1'b1;
    fx1 = t;
  endfunction
  localparam int Q = fd(2) - fx1(2);
  localparam int P = fd(2);
  localparam int R = P - fx1(2);
  initial begin #1 $display("Q=%0d P=%0d R=%0d", Q, P, R); $finish; end
  initial #100 $finish;
endmodule
