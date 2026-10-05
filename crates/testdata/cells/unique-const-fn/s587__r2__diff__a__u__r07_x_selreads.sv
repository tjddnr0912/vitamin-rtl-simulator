module top;
  typedef struct packed { logic [1:0] a; logic [1:0] b; } s_t;
  function automatic logic [3:0] f1(input int a);
    logic [3:0] t;
    unique if (a == 1) t = 4'd1;
    f1 = {3'b000, t[0]};
  endfunction
  function automatic logic [3:0] f2(input int a);
    logic [3:0] t;
    unique if (a == 1) t = 4'd1;
    f2 = {2'b00, t[1:0]};
  endfunction
  function automatic logic [3:0] f3(input int a);
    s_t s;
    unique if (a == 1) s = 4'd1;
    f3 = {2'b00, s.a};
  endfunction
  function automatic logic [3:0] f4(input int a);
    f4 = 4'd0;
    begin
      logic [3:0] t;
      unique if (a == 1) t = 4'd1;
      f4 = t;
    end
  endfunction
  function automatic logic [3:0] f5(input int a);
    reg [3:0] t;
    unique if (a == 1) t = 4'd1;
    f5 = t;
  endfunction
  function automatic time f6(input int a);
    unique if (a == 1) f6 = 5;
  endfunction
  function automatic [3:0] f7(input int a);
    unique if (a == 1) f7 = 4'd1;
  endfunction
  function automatic f8(input int a);
    unique if (a == 1) f8 = 1'b1;
  endfunction
  function automatic logic [3:0] f9(input int a);
    logic [3:0] arr [2];
    unique if (a == 1) arr[0] = 4'd1;
    f9 = arr[0];
  endfunction
  localparam logic [3:0] P1 = f1(2);
  localparam logic [3:0] P2 = f2(2);
  localparam logic [3:0] P3 = f3(2);
  localparam logic [3:0] P4 = f4(2);
  localparam logic [3:0] P5 = f5(2);
  localparam time P6 = f6(2);
  localparam logic [3:0] P7 = f7(2);
  localparam logic P8 = f8(2);
  localparam logic [3:0] P9 = f9(2);
  initial begin #1 $display("P1=%b P2=%b P3=%b P4=%b P5=%b P6=%0d P7=%b P8=%b P9=%b", P1, P2, P3, P4, P5, P6, P7, P8, P9); $finish; end
endmodule
