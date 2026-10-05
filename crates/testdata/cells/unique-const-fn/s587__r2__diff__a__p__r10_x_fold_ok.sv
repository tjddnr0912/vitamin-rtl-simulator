module top;
  function automatic logic [3:0] h2(input int a);
    logic [3:0] t;
    for (int i = 0; i < 4; i++) if (i == a) t = i;
    if (a == 1) t = 4'd9;
    h2 = t;
  endfunction
  function automatic bit [3:0] fbit(input int a);
    if (a == 1) fbit = 4'd1;
  endfunction
  function automatic integer fk(input int a);
    integer k;
    k = 5;
    if (a == 1) k = 1;
    return k;
  endfunction
  localparam logic [3:0] S1 = h2(2);
  localparam logic [3:0] S2 = fbit(2);
  localparam integer S3 = fk(2);
  initial begin #1 $display("S1=%b S2=%b S3=%0d", S1, S2, S3); $finish; end
endmodule
