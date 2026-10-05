module top;
  function automatic logic [3:0] g1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    g1 = t & 4'b0000;
  endfunction
  function automatic int g2(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    g2 = $bits(t);
  endfunction
  function automatic logic g3(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    g3 = (t === t);
  endfunction
  localparam logic [3:0] Q1 = g1(2);
  localparam int Q2 = g2(2);
  localparam logic Q3 = g3(2);
  initial begin #1 $display("Q1=%b Q2=%0d Q3=%b", Q1, Q2, Q3); $finish; end
endmodule
