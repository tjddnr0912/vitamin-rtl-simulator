module top;
  logic [7:0] v;
  logic signed [7:0] s;
  int m;
  task automatic t1(input logic [7:0] x);
    v = x;
    case (v) inside
      (-4'sd8 + 8'sd0): m = 1;
      default: m = 0;
    endcase
    $display("A v=%h m=%0d", v, m);
  endtask
  task automatic t2(input logic signed [7:0] x);
    s = x;
    case (s) inside
      (-4'sd8 + 8'sd0): m = 1;
      default: m = 0;
    endcase
    $display("B s=%h m=%0d", s, m);
  endtask
  initial begin
    #10 $finish;
  end
  initial begin
    t1(8'hF8); t1(8'h08);
    t2(8'shF8); t2(8'sh08);
    v = 8'hF8; $display("C %0d", v inside {(-4'sd8 + 8'sd0)});
    v = 8'h08; $display("D %0d", v inside {(-4'sd8 + 8'sd0)});
    $finish;
  end
endmodule
