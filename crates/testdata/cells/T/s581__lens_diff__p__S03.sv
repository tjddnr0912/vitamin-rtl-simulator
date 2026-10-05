module top;
  localparam [3:0] A4 = 4'b1100;
  localparam int AR [3] = '{1, 2, 3};
  localparam I1 = (5 inside {[4:$]});
  localparam I2 = (2 inside {AR});
  localparam I3 = ((2 inside {1, 2}) inside {1'b1});
  localparam I4 = (2.5 inside {[1.0:3.0]});
  localparam I5 = (2.5 inside {2, 3});
  localparam I6 = (3.0 inside {3});
  localparam I7 = (-2 inside {[-3:-1]});
  localparam I8 = (4'sb1110 inside {[-3:-1]});
  localparam I9 = (4'b1110 inside {[-3:-1]});
  localparam IA = (8'hFF inside {4'hF, 16'hFF});
  localparam IB = (A4 inside {[0:3], 4'b11?1});
  localparam IC = (A4 inside {[0:3], 4'b11?0});
  initial $display("@I %b%b%b%b%b%b%b%b%b%b%b%b", I1, I2, I3, I4, I5, I6, I7, I8, I9, IA, IB, IC);
  initial $display("@R %b%b%b%b%b%b%b%b%b%b%b%b", (5 inside {[4:$]}), (2 inside {AR}), ((2 inside {1, 2}) inside {1'b1}), (2.5 inside {[1.0:3.0]}), (2.5 inside {2, 3}), (3.0 inside {3}), (-2 inside {[-3:-1]}), (4'sb1110 inside {[-3:-1]}), (4'b1110 inside {[-3:-1]}), (8'hFF inside {4'hF, 16'hFF}), (A4 inside {[0:3], 4'b11?1}), (A4 inside {[0:3], 4'b11?0}));
endmodule
