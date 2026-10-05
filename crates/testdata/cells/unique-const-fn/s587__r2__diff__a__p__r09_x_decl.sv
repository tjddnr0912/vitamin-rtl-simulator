module top;
  function automatic logic [3:0] h1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    h1 = t * 4'd0;
  endfunction
  function automatic logic [3:0] h2(input int a);
    logic [3:0] t;
    for (int i = 0; i < 4; i++) if (i == a) t = i;
    if (a == 1) t = 4'd9;
    h2 = t;
  endfunction
  function automatic logic [3:0] h3(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    return t;
  endfunction
  function automatic logic [3:0] h4(input int n);
    logic [3:0] t;
    if (n == 99) t = 4'd1;
    if (n > 0) h4 = h4(n - 1) + 4'd1;
    else h4 = t;
  endfunction
  function automatic logic [3:0] idn(input logic [3:0] v);
    return v | 4'b0000;
  endfunction
  function automatic logic [3:0] h5(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd1;
    h5 = idn(t);
  endfunction
  localparam logic [3:0] R1 = h1(2);
  localparam logic [3:0] R2 = h2(7);
  localparam logic [3:0] R3 = h3(2);
  localparam logic [3:0] R4 = h4(2);
  localparam logic [3:0] R5 = h5(2);
  initial begin #1 $display("R1=%b R2=%b R3=%b R4=%b R5=%b", R1, R2, R3, R4, R5); $finish; end
endmodule
